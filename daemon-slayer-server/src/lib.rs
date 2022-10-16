mod handler;

mod service;
pub use service::*;

pub mod platform;

#[cfg(feature = "async-tokio")]
mod async_impl;

#[cfg(feature = "async-tokio")]
pub use {crate::handler::HandlerAsync as Handler, async_impl::*};

#[cfg(feature = "blocking")]
mod blocking_impl;

#[cfg(feature = "blocking")]
pub mod blocking {
    pub use crate::{blocking_impl::*, handler::HandlerSync as Handler};
}

#[cfg(target_os = "linux")]
pub use sd_notify;

#[cfg(windows)]
pub use windows_service;

#[cfg(all(feature = "async-tokio", feature = "ipc-health-check"))]
mod ipc_health_check;
#[cfg(all(feature = "async-tokio", feature = "ipc-health-check"))]
pub use ipc_health_check::*;

#[cfg(feature = "async-tokio")]
pub use async_trait;
#[cfg(feature = "async-tokio")]
pub use futures;
#[cfg(feature = "async-tokio")]
pub use tokio;

pub use daemon_slayer_core::{BroadcastEventStore, EventStore, Receiver};
pub use daemon_slayer_macros::*;
pub use maybe_async_cfg;
pub use tracing;
