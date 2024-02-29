use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::BroadcastEventStore;
use daemon_slayer_core::{async_trait, BoxedError, FutureExt};
use futures::StreamExt;
use net_route::RouteChange;
use tokio::sync::broadcast;
use tracing::info;

pub struct RouteListenerService {
    event_tx: broadcast::Sender<RouteChange>,
}

impl RouteListenerService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(32);
        Self { event_tx }
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<RouteChange> {
        BroadcastEventStore::new(self.event_tx.clone())
    }
}

#[async_trait]
impl BackgroundService for RouteListenerService {
    fn name(&self) -> &str {
        "route_listener_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let handle = net_route::Handle::new().unwrap();
        let stream = handle.route_listen_stream();

        futures::pin_mut!(stream);

        info!("HERE");
        while let Ok(Some(value)) = stream
            .next()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            info!("route change {value:?}");
            self.event_tx.send(value).ok();
        }
        info!("HERE2");
        Ok(())
    }
}
