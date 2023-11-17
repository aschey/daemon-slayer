use std::collections::HashMap;
use std::fmt::Display;

use recap::Recap;
use serde::{Deserialize, Serialize};

mod mdns_broadcast;
pub use mdns_broadcast::*;
mod mdns_query;
pub use mdns_query::*;
mod udp_broadcast;
pub use udp_broadcast::*;
mod udp_query;
pub use udp_query::*;
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
