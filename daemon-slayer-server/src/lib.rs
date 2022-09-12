mod handler;
pub use handler::*;
mod service;
pub use service::*;
mod event;
pub use event::*;
mod signal;
pub use signal::*;

#[cfg(target_os = "linux")]
pub use sd_notify;

#[cfg(windows)]
pub use windows_service;

#[cfg(all(
    any(feature = "signal-handler-blocking", feature = "signal-handler-async"),
    any(unix, feature = "direct")
))]
pub use signal_hook;
#[cfg(all(unix, feature = "signal-handler-async"))]
pub use signal_hook_tokio;
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

pub use daemon_slayer_macros::*;
pub use maybe_async_cfg;
pub use tracing;
