use std::collections::HashMap;
use std::fmt::Display;
use std::net::IpAddr;
use std::time::Duration;

use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::BroadcastEventStore;
use daemon_slayer_core::{async_trait, BoxedError, FutureExt};
use gethostname::gethostname;
use if_addrs::IfAddr;
use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceInfo, UnregisterStatus};
use recap::Recap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{get_default_ip, ServiceMetadata, ServiceProtocol};

#[derive(Deserialize, Serialize, Debug, Recap)]
#[recap(
    regex = r"^(?P<instance_name>[a-zA-Z0-9_-]+)\.((?P<subdomain>[a-zA-Z0-9_-]+)\._sub.)?_(?P<type_name>[a-zA-Z0-9_-]+)\._(?P<service_protocol>(?:tcp)|(?:udp))\.local\.$"
)]
pub struct MdnsBroadcastName {
    instance_name: String,
    subdomain: Option<String>,
    type_name: String,
    service_protocol: ServiceProtocol,
}

impl MdnsBroadcastName {
    pub fn new(
        instance_name: impl Into<String>,
        type_name: impl Into<String>,
        service_protocol: ServiceProtocol,
    ) -> Self {
        Self {
            subdomain: None,
            instance_name: instance_name.into(),
            type_name: type_name.into(),
            service_protocol,
        }
    }

    pub fn with_subdomain(self, subdomain: impl Into<String>) -> Self {
        Self {
            subdomain: Some(subdomain.into()),
            ..self
        }
    }

    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    pub fn service_type(&self) -> String {
        let service_type = format!("_{}._{}.local.", self.type_name, self.service_protocol);
        if let Some(subdomain) = &self.subdomain {
            format!("{subdomain}._sub.{service_type}")
        } else {
            service_type
        }
    }
}

impl Display for MdnsBroadcastName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}.{}", self.instance_name, self.service_type()))
    }
}

#[derive(Clone, Debug)]
pub enum MdnsBroadcastEvent {
    Announce {
        service_name: String,
        addresses: String,
    },
    IpAdd(IpAddr),
    IpDel(IpAddr),
    ParseIpAddrError(String),
    Error(String),
    Unregistered,
    RegistrationMissing,
}

pub struct MdnsBroadcastService {
    service_name: MdnsBroadcastName,
    event_tx: broadcast::Sender<MdnsBroadcastEvent>,
    broadcast_data: HashMap<String, String>,
    port: u16,
}

impl MdnsBroadcastService {
    pub fn new(
        service_name: MdnsBroadcastName,
        port: u16,
        broadcast_data: impl ServiceMetadata,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(32);
        Self {
            service_name,
            event_tx,
            broadcast_data: broadcast_data.metadata(),
            port,
        }
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<MdnsBroadcastEvent> {
        BroadcastEventStore::new(self.event_tx.clone())
    }
}

#[async_trait]
impl BackgroundService for MdnsBroadcastService {
    fn shutdown_timeout() -> Duration {
        Duration::from_secs(1)
    }

    fn name(&self) -> &str {
        "mdns_broadcast_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let address = get_default_ip()
            .await?
            .map(|ip| ip.to_string())
            .unwrap_or_default();

        let hostname = gethostname().to_string_lossy().to_string();
        let mdns = ServiceDaemon::new()?;
        let mut service_info = ServiceInfo::new(
            &self.service_name.service_type(),
            self.service_name.instance_name(),
            &hostname,
            &address,
            self.port,
            self.broadcast_data,
        )?;

        if address.is_empty() {
            service_info = service_info.enable_addr_auto();
        }

        let monitor = mdns.monitor().unwrap();
        let service_fullname = service_info.get_fullname().to_owned();
        mdns.register(service_info)
            .expect("Failed to register mDNS service");

        while let Ok(Ok(event)) = monitor
            .recv_async()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            match event {
                DaemonEvent::Announce(service_name, addresses) => {
                    self.event_tx
                        .send(MdnsBroadcastEvent::Announce {
                            service_name,
                            addresses,
                        })
                        .ok();
                }
                DaemonEvent::IpAdd(addr) => {
                    self.event_tx.send(MdnsBroadcastEvent::IpAdd(addr)).ok();
                }
                DaemonEvent::IpDel(addr) => {
                    self.event_tx.send(MdnsBroadcastEvent::IpDel(addr)).ok();
                }
                DaemonEvent::Error(mdns_sd::Error::ParseIpAddr(err)) => {
                    self.event_tx
                        .send(MdnsBroadcastEvent::ParseIpAddrError(err))
                        .ok();
                }
                DaemonEvent::Error(mdns_sd::Error::Msg(err)) => {
                    self.event_tx.send(MdnsBroadcastEvent::Error(err)).ok();
                }
                _ => {}
            }
        }

        let receiver = mdns.unregister(&service_fullname).unwrap();
        while let Ok(event) = receiver.recv_async().await {
            match event {
                UnregisterStatus::OK => {
                    self.event_tx.send(MdnsBroadcastEvent::Unregistered).ok();
                }
                UnregisterStatus::NotFound => {
                    self.event_tx
                        .send(MdnsBroadcastEvent::RegistrationMissing)
                        .ok();
                }
            }
        }

        Ok(())
    }
}
