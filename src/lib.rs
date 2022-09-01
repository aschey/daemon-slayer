pub mod service;

#[cfg(any(unix, feature = "direct"))]
pub use signal_hook;
#[cfg(all(unix, feature = "async-tokio"))]
pub use signal_hook_tokio;

#[cfg(windows)]
pub use windows_service;

#[cfg(target_os = "linux")]
pub use sd_notify;

#[cfg(feature = "async-tokio")]
pub use async_trait;
#[cfg(feature = "async-tokio")]
pub use futures;
#[cfg(feature = "async-tokio")]
pub use tokio;

pub use maybe_async;
pub use paste;
pub use tracing;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "console")]
pub mod console;
#[cfg(feature = "logging")]
pub mod logging;
