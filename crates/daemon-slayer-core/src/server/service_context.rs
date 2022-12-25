use crate::BoxedError;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::RwLock,
    task::{JoinError, JoinHandle},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use super::BackgroundService;

struct ServiceInfo {
    name: String,
    timeout: Duration,
    handle: JoinHandle<Result<(), BoxedError>>,
}

#[derive(thiserror::Error, Debug)]
#[error("Some background services failed to execute: {0:?}")]
pub struct BackgroundServiceErrors(pub Vec<BackgroundServiceError>);

#[derive(thiserror::Error, Debug)]
pub enum BackgroundServiceError {
    #[error("Service {0} failed to shut down within the timeout")]
    TimedOut(String),
    #[error("Service {0} encountered an error: {1:?}")]
    ExecutionFailure(String, BoxedError),
    #[error("Service {0} panicked: {1}")]
    ExecutionPanic(String, JoinError),
}

#[derive(thiserror::Error, Debug)]
pub enum AddServiceError {
    #[error("Unable to add the service because the service manager has already been stopped")]
    ManagerStopped,
}

pub struct BackgroundServiceManager {
    cancellation_token: CancellationToken,
    services: Arc<RwLock<Option<Vec<ServiceInfo>>>>,
}

impl BackgroundServiceManager {
    pub fn new(cancellation_token: CancellationToken) -> Self {
        Self {
            services: Arc::new(RwLock::new(Some(vec![]))),
            cancellation_token,
        }
    }

    pub async fn stop(self) -> Result<(), BackgroundServiceErrors> {
        self.cancellation_token.cancel();
        let mut errors = vec![];
        if let Some(services) = self.services.write().await.take() {
            for service in services {
                match tokio::time::timeout(service.timeout, service.handle).await {
                    Ok(Ok(Ok(_))) => info!("Worker {} shutdown successfully", service.name),
                    Ok(Ok(Err(e))) => errors.push(BackgroundServiceError::ExecutionFailure(
                        service.name.to_owned(),
                        e,
                    )),
                    Ok(Err(e)) => errors.push(BackgroundServiceError::ExecutionPanic(
                        service.name.to_owned(),
                        e,
                    )),
                    Err(_) => {
                        errors.push(BackgroundServiceError::TimedOut(service.name.to_owned()))
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(BackgroundServiceErrors(errors))
        }
    }

    pub fn get_context(&self) -> ServiceContext {
        ServiceContext {
            cancellation_token: self.cancellation_token.child_token(),
            services: self.services.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ServiceContext {
    cancellation_token: CancellationToken,
    services: Arc<RwLock<Option<Vec<ServiceInfo>>>>,
}

impl ServiceContext {
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.child_token()
    }

    pub async fn add_service<S: BackgroundService + 'static>(
        &mut self,
        service: S,
    ) -> Result<(), AddServiceError> {
        if let Some(services) = &mut *self.services.write().await {
            let context = self.clone();
            let handle = tokio::spawn(async move { service.run(context).await });
            services.push(ServiceInfo {
                handle,
                name: S::name().to_owned(),
                timeout: S::shutdown_timeout(),
            });
            Ok(())
        } else {
            Err(AddServiceError::ManagerStopped)
        }
    }
}
