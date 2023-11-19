use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::net::IpAddr;

use daemon_slayer_core::BoxedError;
use if_addrs::IfAddr;
use ipnet::{Ipv4Net, Ipv6Net};
use net_route::Route;
use recap::Recap;
use serde::{Deserialize, Serialize};

#[cfg(feature = "cli")]
pub mod cli;
pub mod discovery;
pub mod mdns;
pub mod udp;
pub use {bytes, futures, serde_json, tokio_util};

#[derive(Deserialize, Serialize, Debug, Recap, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServiceProtocol {
    #[recap(regex = r"^tcp$")]
    #[default]
    Tcp,
    #[recap(regex = r"^udp$")]
    Udp,
}

impl Display for ServiceProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp => f.write_str("tcp"),
            Self::Udp => f.write_str("udp"),
        }
    }
}

pub trait ServiceMetadata {
    fn metadata(&self) -> HashMap<String, String>;
    fn from_metadata(metadata: HashMap<String, String>) -> Self;
}

impl ServiceMetadata for HashMap<String, String> {
    fn metadata(&self) -> HashMap<String, String> {
        self.clone()
    }

    fn from_metadata(metadata: HashMap<String, String>) -> Self {
        metadata
    }
}

impl ServiceMetadata for Option<HashMap<String, String>> {
    fn metadata(&self) -> HashMap<String, String> {
        HashMap::default()
    }

    fn from_metadata(metadata: HashMap<String, String>) -> Self {
        Some(metadata)
    }
}

pub(crate) async fn get_default_route() -> Result<Option<Route>, std::io::Error> {
    let net_handle = net_route::Handle::new()?;
    net_handle.default_route().await
}

pub(crate) async fn get_default_ip() -> Result<Option<IpAddr>, BoxedError> {
    let route = get_default_route().await?;
    if let Some(route) = route {
        get_default_ip_from_route(&route)
    } else {
        Ok(None)
    }
}

pub(crate) fn get_default_ip_from_route(route: &Route) -> Result<Option<IpAddr>, BoxedError> {
    if let Some(default_route) = route.gateway {
        // Try to find the address that matches the default route
        // so we don't accidentally broadcast an internal IP
        for interface in if_addrs::get_if_addrs()? {
            if is_address_in_route(&interface.addr, &default_route) {
                return Ok(Some(interface.addr.ip()));
            }
        }
    }

    Ok(None)
}

// TODO: is this the best way to do this?
fn is_address_in_route(if_addr: &IfAddr, default_route: &IpAddr) -> bool {
    match (if_addr, default_route) {
        (IfAddr::V4(v4addr), IpAddr::V4(default_addr)) => {
            if let Some(broadcast) = v4addr.broadcast {
                let mask = v4addr
                    .netmask
                    .octets()
                    .into_iter()
                    .take_while(|i| *i == 255)
                    .count()
                    * 8;
                if let Ok(net) = Ipv4Net::new(*default_addr, mask as u8) {
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
                if let Ok(net) = Ipv6Net::new(*default_addr, mask as u8) {
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

#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Eq, Recap, Clone)]
#[recap(
    regex = r"^(?P<instance_name>[a-zA-Z0-9_-]+)\.((?P<subdomain>[a-zA-Z0-9_-]+)\.)?(?P<type_name>[a-zA-Z0-9_-]+)$"
)]
pub struct BroadcastServiceName {
    instance_name: String,
    subdomain: Option<String>,
    type_name: String,
}

impl BroadcastServiceName {
    pub fn new(instance_name: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            subdomain: None,
            instance_name: instance_name.into(),
            type_name: type_name.into(),
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

    pub fn subdomain(&self) -> Option<&str> {
        self.subdomain.as_deref()
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }
}

#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Eq, Recap, Clone)]
#[recap(regex = r"^((?P<subdomain>[a-zA-Z0-9_-]+))?(?P<type_name>[a-zA-Z0-9_-]+)$")]
pub struct QueryServiceName {
    subdomain: Option<String>,
    type_name: String,
}

impl QueryServiceName {
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            subdomain: None,
        }
    }

    pub fn with_subdomain(self, subdomain: impl Into<String>) -> Self {
        Self {
            subdomain: Some(subdomain.into()),
            ..self
        }
    }

    pub fn subdomain(&self) -> Option<&str> {
        self.subdomain.as_deref()
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInfo {
    host_name: String,
    service_name: BroadcastServiceName,
    service_protocol: ServiceProtocol,
    port: u16,
    ip_addresses: HashSet<IpAddr>,
    broadcast_data: HashMap<String, String>,
}
