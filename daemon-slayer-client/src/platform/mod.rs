#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::ServiceManager;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use self::mac::ServiceManager;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::ServiceManager;
