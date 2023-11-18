use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use bytes::Bytes;
use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::BroadcastEventStore;
use daemon_slayer_core::{async_trait, BoxedError, FutureExt};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

use super::ServiceInfo;
use crate::ServiceMetadata;

async fn test<M: ServiceMetadata>(metadata: M) {
    let sender = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let dest: SocketAddr = "0.0.0.0:34254".parse().unwrap();
    sender.set_broadcast(true).unwrap();
    let metadata = metadata.metadata();
    let mut framed = UdpFramed::new(sender, BytesCodec::new());
    let json_data = serde_json::to_string(&metadata).unwrap();
    framed.send((Bytes::from(json_data), dest)).await.unwrap();
}

async fn test2<M: ServiceMetadata>() {
    let recv = UdpSocket::bind("0.0.0.0:34254").await.unwrap();
    recv.set_broadcast(true).unwrap();
    let mut framed = UdpFramed::new(recv, BytesCodec::new());
    let (data, sender_addr) = framed.next().await.unwrap().unwrap();

    let metadata: HashMap<String, String> = serde_json::from_slice(&data).unwrap();
    let data = M::from_metadata(metadata);
}

pub struct UdpQueryService {
    broadcast_port: u16,
    event_tx: broadcast::Sender<ServiceInfo>,
}

impl UdpQueryService {
    pub fn new(broadcast_port: u16) -> Self {
        let (event_tx, _) = broadcast::channel(32);
        Self {
            broadcast_port,
            event_tx,
        }
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
