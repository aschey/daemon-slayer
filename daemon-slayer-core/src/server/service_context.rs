use std::{pin::Pin, time::Duration};

use futures::Future;
use tap::TapFallible;
use tokio::task::JoinHandle;
use tokio_graceful_shutdown::SubsystemHandle;
use tracing::warn;

use super::{BackgroundService, EventService};

pub struct ServiceContext {
    subsys: SubsystemHandle,
    handles: Vec<(JoinHandle<()>, Duration)>,
}

impl ServiceContext {
    pub fn new(subsys: SubsystemHandle) -> Self {
        Self {
            handles: vec![],
            subsys,
        }
    }

    pub fn get_subsystem_handle(&self) -> SubsystemHandle {
        self.subsys.clone()
    }

    pub async fn add_event_service<S: EventService + 'static>(
        &mut self,
        mut service: S,
    ) -> (S::Client, S::EventStoreImpl) {
        let client = service.get_client().await;
        let event_store = service.get_event_store();
        let subsys = self.subsys.clone();
        self.handles.push((
            tokio::spawn(async move {
                service.run(subsys).await;
            }),
            S::shutdown_timeout(),
        ));
        (client, event_store)
    }

    pub async fn add_service<S: BackgroundService + 'static>(
        &mut self,
        mut service: S,
    ) -> S::Client {
        let client = service.get_client().await;
        let subsys = self.subsys.clone();
        self.handles.push((
            tokio::spawn(async move {
                service.run(subsys).await;
            }),
            S::shutdown_timeout(),
        ));
        client
    }

    pub async fn stop(self) {
        self.subsys.request_global_shutdown();
        for (handle, timeout) in self.handles {
            match tokio::time::timeout(timeout, handle).await {
                Ok(_) => tracing::info!("Worker shutdown successfully"),
                Err(_) => tracing::warn!("Worker failed to shut down"),
            }
        }
    }
}
