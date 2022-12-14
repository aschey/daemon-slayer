mod handler;
pub use handler::*;

mod service;
pub use service::*;

mod service_error;
pub use service_error::*;

pub mod platform;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(target_os = "linux")]
pub use sd_notify;

#[cfg(windows)]
pub use once_cell;
#[cfg(windows)]
pub use windows_service;

pub use daemon_slayer_core::{
    async_trait,
    server::{BackgroundService, BroadcastEventStore, EventStore, ServiceContext},
    signal::{Client as SignalHandlerClient, Handler as SignalHandler, Signal},
    AsAny,
};
pub use futures;

pub use daemon_slayer_macros::*;
pub use tracing;
