use std::{pin::Pin, sync::Arc, time::Duration};

use futures::Future;
use tap::TapFallible;
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing::warn;

use super::{BackgroundService, EventService};

pub struct ServiceManager {
    subsys: SubsystemHandle,
    handles: Arc<RwLock<Option<Vec<(JoinHandle<()>, Duration)>>>>,
}

impl ServiceManager {
    pub fn new(subsys: SubsystemHandle) -> Self {
        Self {
            handles: Arc::new(RwLock::new(Some(vec![]))),
            subsys,
        }
    }

    pub async fn stop(self) {
        self.subsys.request_global_shutdown();
        if let Some(handles) = self.handles.write().await.take() {
            for (handle, timeout) in handles {
                match tokio::time::timeout(timeout, handle).await {
                    Ok(_) => tracing::info!("Worker shutdown successfully"),
                    Err(_) => tracing::warn!("Worker failed to shut down"),
                }
            }
        }
    }

    pub async fn get_context(&self) -> ServiceContext {
        ServiceContext {
            subsys: self.subsys.clone(),
            handles: self.handles.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ServiceContext {
    subsys: SubsystemHandle,
    handles: Arc<RwLock<Option<Vec<(JoinHandle<()>, Duration)>>>>,
}

impl ServiceContext {
    pub fn get_subsystem_handle(&self) -> SubsystemHandle {
        self.subsys.clone()
    }

    pub async fn add_event_service<S: EventService + 'static>(
        &mut self,
        mut service: S,
    ) -> (S::Client, S::EventStoreImpl) {
        if let Some(handles) = &mut *self.handles.write().await {
            let client = service.get_client().await;
            let event_store = service.get_event_store();
            let context = self.clone();
            handles.push((
                tokio::spawn(async move {
                    service.run(context).await;
                }),
                S::shutdown_timeout(),
            ));
            (client, event_store)
        } else {
            panic!();
        }
    }

    pub async fn add_service<S: BackgroundService + 'static>(
        &mut self,
        mut service: S,
    ) -> S::Client {
        if let Some(handles) = &mut *self.handles.write().await {
            let client = service.get_client().await;
            let context = self.clone();
            handles.push((
                tokio::spawn(async move {
                    service.run(context).await;
                }),
                S::shutdown_timeout(),
            ));
            client
        } else {
            panic!();
        }
    }
}
