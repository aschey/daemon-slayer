use std::io;

use daemon_slayer_core::BoxedError;
use daemon_slayer_core::server::BroadcastEventStore;
use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use futures::{Stream, StreamExt};
use serde::Deserialize;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio_util::codec::BytesCodec;
use tokio_util::future::FutureExt;
use tokio_util::udp::UdpFramed;

use super::DEFAULT_BROADCAST_PORT;
use crate::ServiceInfo;

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

    pub(crate) async fn get_framed(&self) -> impl Stream<Item = Result<ServiceInfo, io::Error>> {
        let receiver = UdpSocket::bind(format!("0.0.0.0:{}", self.broadcast_port))
            .await
            .unwrap();
        receiver.set_broadcast(true).unwrap();

        UdpFramed::new(receiver, BytesCodec::new()).map(|item| {
            let (data, _) = item?;
            let mut deserializer = serde_json::Deserializer::from_slice(&data);
            Ok(ServiceInfo::deserialize(&mut deserializer).unwrap())
        })
    }
}

impl BackgroundService for UdpQueryService {
    fn name(&self) -> &str {
        "udp_query_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut framed = self.get_framed().await;

        let mut last_result = ServiceInfo::default();
        while let Some(Ok(service_info)) = framed
            .next()
            .with_cancellation_token(context.cancellation_token())
            .await
            .flatten()
        {
            if service_info != last_result {
                self.event_tx.send(service_info.clone()).unwrap();
                last_result = service_info;
            }
        }

        Ok(())
    }
}
