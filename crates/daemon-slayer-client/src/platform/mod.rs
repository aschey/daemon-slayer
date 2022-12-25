#[cfg(target_os = "macos")]
mod launchd;
use std::io;

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

use crate::{
    config::{Builder, Program},
    Manager,
};

pub fn builder(label: Label, program: Program) -> Builder {
    Builder::new(label, program)
}

pub(crate) fn get_manager(builder: Builder) -> Result<Box<dyn Manager>, io::Error> {
    #[cfg(target_os = "linux")]
    return Ok(Box::new(SystemdServiceManager::from_builder(builder)?));
    #[cfg(windows)]
    return Ok(Box::new(WindowsServiceManager::from_builder(builder)?));
    #[cfg(target_os = "macos")]
    return Ok(Box::new(LaunchdServiceManager::from_builder(builder)?));
}
