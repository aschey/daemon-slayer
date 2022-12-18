#[cfg(target_os = "macos")]
mod launchd;
#[cfg(target_os = "macos")]
use self::launchd::*;
#[cfg(target_os = "linux")]
mod systemd;
#[cfg(target_os = "linux")]
use systemd::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::*;

use daemon_slayer_core::Label;

use crate::{configuration::Builder, Manager};

pub fn builder(label: Label) -> Builder {
    Builder::new(label)
}

pub(crate) fn get_manager(builder: Builder) -> crate::Result<Box<dyn Manager>> {
    #[cfg(target_os = "linux")]
    return Ok(Box::new(SystemdServiceManager::from_builder(builder)?));
    #[cfg(windows)]
    return Ok(Box::new(WindowsServiceManager::from_builder(builder)?));
    #[cfg(target_os = "macos")]
    return Ok(Box::new(LaunchdServiceManager::from_builder(builder)?));
}
