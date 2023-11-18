mod broadcast;
mod query;

pub use broadcast::*;
pub use query::*;

#[derive(Debug, Clone, Copy)]
pub enum DiscoveryProtocol {
    Mdns,
    Udp,
    Both,
}
