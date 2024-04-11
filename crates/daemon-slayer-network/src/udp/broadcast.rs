use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::Duration;

use bytes::Bytes;
use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::BoxedError;
use futures::SinkExt;
use gethostname::gethostname;
use serde::Serialize;
use tap::TapFallible;
use tokio::net::UdpSocket;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;
use tracing::error;

use super::DEFAULT_BROADCAST_PORT;
use crate::{
    get_default_interface, BroadcastServiceName, ServiceInfo, ServiceMetadata, ServiceProtocol,
};

pub struct UdpBroadcastService {
    service_name: BroadcastServiceName,
    service_protocol: ServiceProtocol,
    port: u16,
    broadcast_data: HashMap<String, String>,
    broadcast_interval: Duration,
    broadcast_port: u16,
}

impl UdpBroadcastService {
    pub fn new(
        service_name: BroadcastServiceName,
        service_protocol: ServiceProtocol,
        port: u16,
        broadcast_data: impl ServiceMetadata,
    ) -> Self {
        Self {
            service_name,
            service_protocol,
            port,
            broadcast_data: broadcast_data.metadata(),
            broadcast_interval: Duration::from_millis(5000),
            broadcast_port: DEFAULT_BROADCAST_PORT,
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
}

impl BackgroundService for UdpBroadcastService {
    fn name(&self) -> &str {
        "udp_broadcast_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let sender = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        sender.set_broadcast(true).unwrap();
        let dest: SocketAddr = format!("255.255.255.255:{}", self.broadcast_port)
            .parse()
            .unwrap();
        let mut framed = UdpFramed::new(sender, BytesCodec::new());

        let cancellation_token = context.cancellation_token();
        let ips = match get_default_interface().await? {
            Some(interface) => HashSet::from_iter([interface.ip()]),
            None => if_addrs::get_if_addrs()?
                .into_iter()
                .map(|addr| addr.ip())
                .collect(),
        };
        let service_info = ServiceInfo {
            host_name: gethostname().to_string_lossy().to_string(),
            service_name: self.service_name,
            service_protocol: self.service_protocol,
            port: self.port,
            ip_addresses: ips,
            broadcast_data: self.broadcast_data,
        };
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.broadcast_interval) => {
                    let mut buf = Vec::new();
                    let mut serializer = serde_json::Serializer::new(&mut buf);
                    if service_info
                        .serialize(&mut serializer)
                        .tap_err(|e| error!("error serializing service info {e:?}"))
                        .is_ok()
                    {
                        if let Err(e) = framed.send((Bytes::from(buf), dest)).await {
                            error!("error sending service info: {e:?}");
                        }
                    }
                }
                _ = cancellation_token.cancelled() => {
                    break;
                }
            }
        }
        Ok(())
    }
}
