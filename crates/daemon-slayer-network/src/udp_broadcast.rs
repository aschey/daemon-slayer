use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use bytes::Bytes;
use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::{async_trait, BoxedError};
use futures::SinkExt;
use tokio::net::UdpSocket;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

pub struct UdpBroadcastService {
    broadcast_data: Bytes,
    broadcast_interval: Duration,
    port: u16,
}

#[async_trait]
impl BackgroundService for UdpBroadcastService {
    fn shutdown_timeout() -> Duration {
        Duration::from_secs(1)
    }

    fn name(&self) -> &str {
        "udp_broadcast_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let sender = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        sender.set_broadcast(true).unwrap();
        let dest: SocketAddr = format!("255.255.255.255:{}", self.port).parse().unwrap();
        let mut framed = UdpFramed::new(sender, BytesCodec::new());

        let cancellation_token = context.cancellation_token();
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.broadcast_interval) => {
                    framed
                        .send((self.broadcast_data.clone(), dest))
                        .await
                        .unwrap();
                }
                _ = cancellation_token.cancelled() => {
                    break;
                }
            }
        }
        Ok(())
    }
}
