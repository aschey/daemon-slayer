mod broadcast;
mod query;

pub use broadcast::*;
pub use query::*;

#[derive(Debug, Clone, Copy)]
pub enum DiscoveryProtocol {
    Mdns,
    Udp { port: u16 },
    Both { udp_port: u16 },
}
