use crate::BoxedError;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::RwLock,
    task::{JoinError, JoinHandle},
};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing::{error, info};

use super::{BackgroundService, EventService};

struct ServiceInfo {
    name: String,
    timeout: Duration,
    handle: JoinHandle<Result<(), BoxedError>>,
}

#[derive(thiserror::Error, Debug)]
pub enum BackgroundServiceError {
    #[error("Service {0} failed to shut down within the timeout")]
    TimedOut(String),
    #[error("Service {0} encountered an error: {1:?}")]
    ExecutionFailure(String, BoxedError),
    #[error("Service {0} panicked: {1}")]
    ExecutionPanic(String, JoinError),
}

pub struct ServiceManager {
    subsys: SubsystemHandle,
    services: Arc<RwLock<Option<Vec<ServiceInfo>>>>,
}

impl ServiceManager {
    pub fn new(subsys: SubsystemHandle) -> Self {
        Self {
            services: Arc::new(RwLock::new(Some(vec![]))),
            subsys,
        }
    }

    pub async fn stop(self) -> Vec<BackgroundServiceError> {
        self.subsys.request_global_shutdown();
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
        errors
    }

    pub async fn get_context(&self) -> ServiceContext {
        ServiceContext {
            subsys: self.subsys.clone(),
            services: self.services.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ServiceContext {
    subsys: SubsystemHandle,
    services: Arc<RwLock<Option<Vec<ServiceInfo>>>>,
}

impl ServiceContext {
    pub fn get_subsystem_handle(&self) -> SubsystemHandle {
        self.subsys.clone()
    }

    pub async fn add_event_service<S: EventService + 'static>(
        &mut self,
        mut service: S,
    ) -> (S::Client, S::EventStoreImpl) {
        if let Some(services) = &mut *self.services.write().await {
            let client = service.get_client().await;
            let event_store = service.get_event_store();
            let context = self.clone();
            let handle = tokio::spawn(async move { service.run(context).await });
            services.push(ServiceInfo {
                handle,
                name: S::name().to_owned(),
                timeout: S::shutdown_timeout(),
            });
            (client, event_store)
        } else {
            panic!();
        }
    }

    pub async fn add_service<S: BackgroundService + 'static>(
        &mut self,
        mut service: S,
    ) -> S::Client {
        if let Some(services) = &mut *self.services.write().await {
            let client = service.get_client().await;
            let context = self.clone();
            let handle = tokio::spawn(async move { service.run(context).await });
            services.push(ServiceInfo {
                handle,
                name: S::name().to_owned(),
                timeout: S::shutdown_timeout(),
            });
            client
        } else {
            panic!();
        }
    }
}
