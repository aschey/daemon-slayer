#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(unix)]
pub mod unix_macros;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub mod platform {
    pub use crate::windows::Manager;
}

#[cfg(target_os = "linux")]
pub mod platform {
    pub use crate::linux::Manager;
    pub use crate::unix_macros;
}

#[cfg(target_os = "macos")]
pub mod platform {
    pub use crate::mac::Manager;
}

#[cfg(any(unix, feature = "direct"))]
pub use signal_hook;
#[cfg(all(unix, feature = "async-tokio"))]
pub use signal_hook_tokio;

#[cfg(windows)]
pub use windows_service;

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
#[cfg(feature = "logging")]
pub mod logging;

pub mod service_builder;
pub mod service_manager;
pub mod service_status;
