mod broadcast;
mod query;

use std::collections::HashMap;
use std::net::IpAddr;

pub use broadcast::*;
pub use query::*;
use serde::{Deserialize, Serialize};

use crate::{BroadcastServiceName, ServiceProtocol};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInfo {
    host_name: String,
    service_name: BroadcastServiceName,
    service_protocol: ServiceProtocol,
    port: u16,
    ip_addresses: Vec<IpAddr>,
    broadcast_data: HashMap<String, String>,
}

const DEFAULT_BROADCAST_PORT: u16 = 3535;
