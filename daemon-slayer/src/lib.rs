#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub mod platform {
    pub use crate::windows::Manager;
}

#[cfg(target_os = "linux")]
pub mod platform {
    pub use crate::linux::Manager;
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
#[cfg(unix)]
pub use signal_hook_tokio;
#[cfg(unix)]
pub use tokio;

pub mod service_manager;
pub mod service_status;
