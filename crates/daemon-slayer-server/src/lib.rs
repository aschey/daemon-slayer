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
pub use windows_service;

pub use async_trait;
pub use futures;
pub use once_cell;
pub use tokio;

pub use daemon_slayer_core::{
    server::{BackgroundService, BroadcastEventStore, EventStore, ServiceContext},
    signal::{Client as SignalHandlerClient, Handler as SignalHandler, Signal},
    AsAny,
};

pub use daemon_slayer_macros::*;
pub use tracing;
