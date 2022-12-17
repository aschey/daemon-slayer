use std::{pin::Pin, sync::Arc, time::Duration};

use futures::Future;
use tap::TapFallible;
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing::warn;

use super::{BackgroundService, EventService};

struct ServiceInfo {
    name: String,
    timeout: Duration,
    handle: JoinHandle<()>,
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

    pub async fn stop(self) {
        self.subsys.request_global_shutdown();
        if let Some(services) = self.services.write().await.take() {
            for service in services {
                match tokio::time::timeout(service.timeout, service.handle).await {
                    Ok(_) => tracing::info!("Worker {} shutdown successfully", service.name),
                    Err(_) => tracing::warn!("Worker {} failed to shut down", service.name),
                }
            }
        }
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
            let handle = tokio::spawn(async move {
                service.run(context).await;
            });
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
            let handle = tokio::spawn(async move {
                service.run(context).await;
            });
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
