use std::{pin::Pin, time::Duration};

use futures::Future;
use tap::TapFallible;
use tracing::warn;

// use crate::Signal;

pub struct ServiceContext {
    //signal_tx: tokio::sync::broadcast::Sender<crate::Signal>,
    handles: Vec<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl ServiceContext {
    pub(crate) fn new() -> Self {
        Self {
            //   signal_tx,
            handles: vec![],
        }
    }
    pub async fn add_event_service<S: daemon_slayer_core::server::EventService + 'static>(
        &mut self,
        builder: S::Builder,
    ) -> (S::Client, S::EventStoreImpl) {
        let mut service = S::run_service(builder).await;
        let client = service.get_client();
        let event_store = service.get_event_store();
        self.handles.push(Box::pin(async move {
            service.stop().await;
        }));
        (client, event_store)
    }

    pub async fn add_service<S: daemon_slayer_core::server::Service + 'static>(
        &mut self,
        builder: S::Builder,
    ) -> S::Client {
        let mut service = S::run_service(builder).await;
        let client = service.get_client();

        self.handles.push(Box::pin(async move {
            service.stop().await;
        }));
        client
    }

    pub(crate) async fn stop(self) {
        for handle in self.handles {
            match tokio::time::timeout(Duration::from_secs(10), handle).await {
                Ok(()) => tracing::info!("Worker shutdown successfully"),
                Err(_) => tracing::warn!("Worker failed to shut down"),
            }
        }
    }
}

pub struct ServiceContextSync {}
