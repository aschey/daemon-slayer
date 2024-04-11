#[cfg(feature = "cli")]
pub mod cli;
mod handler;
pub mod platform;
mod service;
mod service_error;
#[cfg(feature = "socket-activation")]
pub mod socket_activation;

pub use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
pub use daemon_slayer_core::server::{BroadcastEventStore, EventStore};
pub use daemon_slayer_core::signal::{
    Client as SignalHandlerClient, Handler as SignalHandler, Signal,
};
pub use daemon_slayer_core::AsAny;
pub use daemon_slayer_macros::*;
pub use handler::*;
#[cfg(target_os = "linux")]
pub use sd_notify;
pub use service::*;
pub use service_error::*;
#[cfg(windows)]
pub use windows_service;
pub use {futures, tokio};
