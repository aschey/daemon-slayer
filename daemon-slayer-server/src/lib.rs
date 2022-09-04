mod handler;
pub use handler::{Handler, StopHandler};
#[cfg(feature = "logging")]
pub mod logging;
#[cfg(feature = "logging")]
pub use logging::{logger_builder::LoggerBuilder, logger_guard::LoggerGuard};
mod service;
pub use service::Service;

#[cfg(target_os = "linux")]
pub use sd_notify;

#[cfg(windows)]
pub use windows_service;

#[cfg(any(unix, feature = "direct"))]
pub use signal_hook;
#[cfg(all(unix, feature = "async-tokio"))]
pub use signal_hook_tokio;

#[cfg(feature = "async-tokio")]
pub use async_trait;
#[cfg(feature = "async-tokio")]
pub use futures;
#[cfg(feature = "async-tokio")]
pub use tokio;

pub use daemon_slayer_macros::*;
pub use maybe_async;
pub use tracing;
