use std::collections::HashMap;
use std::fmt::Display;
use std::net::IpAddr;

use daemon_slayer_core::BoxedError;
use if_addrs::IfAddr;
use recap::Recap;
use serde::{Deserialize, Serialize};

pub mod mdns;
pub mod udp;
pub use {bytes, futures, serde_json, tokio_util};

#[derive(Deserialize, Serialize, Debug, Recap)]
pub enum ServiceProtocol {
    #[recap(regex = r"^tcp$")]
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

pub(crate) async fn get_default_ip() -> Result<Option<IpAddr>, BoxedError> {
    let net_handle = net_route::Handle::new()?;

    let default_route = net_handle
        .default_route()
        .await?
        .clone()
        .and_then(|r| r.gateway);

    if let Some(default_route) = default_route {
        // Try to find the address that matches the default route
        // so we don't accidentally broadcast an internal IP
        for iface in if_addrs::get_if_addrs()? {
            if is_address_in_route(&iface.addr, &default_route) {
                return Ok(Some(iface.addr.ip()));
            }
        }
    }

    Ok(None)
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
