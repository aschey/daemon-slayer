mod handler;
pub use handler::*;

mod service;
pub use service::*;

mod service_error;
pub use service_error::*;

pub mod platform;

pub mod socket_activation;

#[cfg(feature = "cli")]
pub mod cli;

pub use daemon_slayer_core::server::{
    BackgroundService, BroadcastEventStore, EventStore, ServiceContext,
};
pub use daemon_slayer_core::signal::{
    Client as SignalHandlerClient, Handler as SignalHandler, Signal,
};
pub use daemon_slayer_core::{async_trait, AsAny};
pub use daemon_slayer_macros::*;
#[cfg(target_os = "linux")]
pub use sd_notify;
#[cfg(windows)]
pub use windows_service;
pub use {futures, tokio};
