#[cfg(target_os = "linux")]
mod systemd;
#[cfg(target_os = "linux")]
pub use self::systemd::*;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use self::mac::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;
