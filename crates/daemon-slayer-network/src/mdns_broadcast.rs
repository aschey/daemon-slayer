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

use crate::{ServiceMetadata, ServiceProtocol};

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
    host_name: String,
    event_tx: broadcast::Sender<MdnsBroadcastEvent>,
    broadcast_data: HashMap<String, String>,
}

impl MdnsBroadcastService {
    pub fn new(service_name: MdnsBroadcastName, broadcast_data: impl ServiceMetadata) -> Self {
        let (event_tx, _) = broadcast::channel(32);
        Self {
            service_name,
            host_name: gethostname().to_string_lossy().to_string(),
            event_tx,
            broadcast_data: broadcast_data.metadata(),
        }
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<MdnsBroadcastEvent> {
        BroadcastEventStore::new(self.event_tx.clone())
    }
}

// TODO: is this the best way to do this?
fn is_address_in_route(ifaddr: &IfAddr, default_route: &IpAddr) -> bool {
    match (ifaddr, default_route) {
        (IfAddr::V4(v4addr), IpAddr::V4(default_addr)) => {
            if let Some(broadcast) = v4addr.broadcast {
                let mask = v4addr
                    .netmask
                    .octets()
                    .into_iter()
                    .take_while(|i| *i == 255)
                    .count()
                    * 8;
                if let Ok(net) = ipnet::Ipv4Net::new(*default_addr, mask as u8) {
                    return net.broadcast() == broadcast;
                }
            }
        }
        (IfAddr::V6(v6addr), IpAddr::V6(default_addr)) => {
            // TODO: not sure if this is correct for IPV6
            if let Some(broadcast) = v6addr.broadcast {
                let mask = v6addr
                    .netmask
                    .octets()
                    .into_iter()
                    .take_while(|i| *i == 255)
                    .count()
                    * 8;
                if let Ok(net) = ipnet::Ipv6Net::new(*default_addr, mask as u8) {
                    return net.broadcast() == broadcast;
                }
            }
        }
        _ => {
            return false;
        }
    }
    false
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
        let net_handle = net_route::Handle::new()?;

        let default_route = net_handle
            .default_route()
            .await?
            .clone()
            .and_then(|r| r.gateway);

        let mut address = String::default();
        if let Some(default_route) = default_route {
            // Try to find the address that matches the default route
            // so we don't accidentally broadcast an internal IP
            for iface in if_addrs::get_if_addrs()? {
                if is_address_in_route(&iface.addr, &default_route) {
                    address = iface.addr.ip().to_string();
                }
            }
        }

        let mdns = ServiceDaemon::new()?;
        let mut service_info = ServiceInfo::new(
            &self.service_name.service_type(),
            self.service_name.instance_name(),
            &self.host_name,
            &address,
            3456,
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
