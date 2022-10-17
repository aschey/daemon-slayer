mod handler;

mod service;
pub use service::*;

pub mod platform;

#[cfg(feature = "async-tokio")]
mod async_context;

#[cfg(feature = "async-tokio")]
pub use {crate::handler::HandlerAsync as Handler, async_context::*};

#[cfg(feature = "blocking")]
mod blocking_context;

#[cfg(feature = "blocking")]
pub mod blocking {
    pub use crate::{blocking_context::*, handler::HandlerSync as Handler};
    pub use daemon_slayer_core::blocking::{BroadcastEventStore, EventStore, Receiver};
}

#[cfg(target_os = "linux")]
pub use sd_notify;

#[cfg(windows)]
pub use windows_service;

#[cfg(feature = "async-tokio")]
pub use async_trait;
#[cfg(feature = "async-tokio")]
pub use futures;
#[cfg(feature = "async-tokio")]
pub use tokio;

#[cfg(feature = "async-tokio")]
pub use daemon_slayer_core::{BroadcastEventStore, EventStore, Receiver};

pub use daemon_slayer_macros::*;
pub use maybe_async_cfg;
pub use tracing;
