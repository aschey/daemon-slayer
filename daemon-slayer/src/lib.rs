#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(unix)]
pub mod unix_macros;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub mod windows_macros;

#[cfg(windows)]
pub mod platform {
    pub use crate::windows::Manager;
    pub use crate::windows_macros;
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

#[cfg(windows)]
pub use ctrlc;
pub use paste;
#[cfg(windows)]
pub use windows_service;

#[cfg(unix)]
pub use futures;
#[cfg(unix)]
pub use signal_hook;
#[cfg(all(unix, feature = "async-tokio"))]
pub use signal_hook_tokio;
#[cfg(feature = "async-tokio")]
pub use tokio;

pub mod service_config;
pub mod service_manager;
pub mod service_status;
