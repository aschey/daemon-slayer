use std::time::Duration;

use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::BroadcastEventStore;
use daemon_slayer_core::{async_trait, BoxedError, FutureExt};
use futures::StreamExt;
use serde::Deserialize;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

use super::{ServiceInfo, DEFAULT_BROADCAST_PORT};

pub struct UdpQueryService {
    broadcast_port: u16,
    event_tx: broadcast::Sender<ServiceInfo>,
}

impl Default for UdpQueryService {
    fn default() -> Self {
        Self::new()
    }
}

impl UdpQueryService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(32);
        Self {
            broadcast_port: DEFAULT_BROADCAST_PORT,
            event_tx,
        }
    }

    pub fn with_broadcast_port(self, broadcast_port: u16) -> Self {
        Self {
            broadcast_port,
            ..self
        }
    }

    pub fn get_broadcast_port(&self) -> u16 {
        self.broadcast_port
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<ServiceInfo> {
        BroadcastEventStore::new(self.event_tx.clone())
    }
}

#[async_trait]
impl BackgroundService for UdpQueryService {
    fn shutdown_timeout() -> Duration {
        Duration::from_secs(1)
    }

    fn name(&self) -> &str {
        "udp_broadcast_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let receiver = UdpSocket::bind(format!("0.0.0.0:{}", self.broadcast_port))
            .await
            .unwrap();
        receiver.set_broadcast(true).unwrap();

        let mut framed = UdpFramed::new(receiver, BytesCodec::new());

        let mut last_result = ServiceInfo::default();
        while let Ok(Some(Ok((data, _)))) = framed
            .next()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            let mut deserializer = serde_json::Deserializer::from_slice(&data);
            let service_info = ServiceInfo::deserialize(&mut deserializer).unwrap();

            if service_info != last_result {
                self.event_tx.send(service_info.clone()).unwrap();
                last_result = service_info;
            }
        }

        Ok(())
    }
}
