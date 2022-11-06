use std::{pin::Pin, time::Duration};

use daemon_slayer_core::server::SubsystemHandle;
use futures::Future;
use tap::TapFallible;
use tracing::warn;

// use crate::Signal;

pub struct ServiceContext {
    subsys: SubsystemHandle,
    handles: Vec<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl ServiceContext {
    pub(crate) fn new(subsys: SubsystemHandle) -> Self {
        Self {
            handles: vec![],
            subsys,
        }
    }

    pub fn get_subsystem_handle(&self) -> SubsystemHandle {
        self.subsys.clone()
    }

    pub async fn add_event_service<S: daemon_slayer_core::server::EventService + 'static>(
        &mut self,
        builder: S::Builder,
    ) -> (S::Client, S::EventStoreImpl) {
        let mut service = S::run_service(builder, self.subsys.clone()).await;
        let client = service.get_client();
        let event_store = service.get_event_store();
        self.handles.push(Box::pin(async move {
            service.stop().await;
        }));
        (client, event_store)
    }

    pub async fn add_service<S: daemon_slayer_core::server::BackgroundService + 'static>(
        &mut self,
        builder: S::Builder,
    ) -> S::Client {
        let mut service = S::run_service(builder, self.subsys.clone()).await;
        let client = service.get_client();

        self.handles.push(Box::pin(async move {
            service.stop().await;
        }));
        client
    }

    pub(crate) async fn stop(self) {
        self.subsys.request_global_shutdown();
        for handle in self.handles {
            match tokio::time::timeout(Duration::from_secs(10), handle).await {
                Ok(()) => tracing::info!("Worker shutdown successfully"),
                Err(_) => tracing::warn!("Worker failed to shut down"),
            }
        }
    }
}

pub struct ServiceContextSync {}
