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
#[cfg(feature = "docker")]
mod docker;
use daemon_slayer_core::Label;
#[cfg(feature = "docker")]
use docker::*;

use crate::config::{Builder, Program, ServiceType};
use crate::ServiceManager;

pub fn builder(label: Label, program: Program) -> Builder {
    Builder::new(label, program)
}

pub(crate) async fn get_manager(builder: Builder) -> io::Result<ServiceManager> {
    #[cfg(feature = "docker")]
    if builder.service_type == ServiceType::Container {
        return Ok(ServiceManager::new(
            DockerServiceManager::from_builder(builder).await?,
        ));
    }

    #[cfg(target_os = "linux")]
    return Ok(ServiceManager::new(
        SystemdServiceManager::from_builder(builder).await?,
    ));
    #[cfg(windows)]
    return Ok(ServiceManager::new(WindowsServiceManager::from_builder(
        builder,
    )?));
    #[cfg(target_os = "macos")]
    return Ok(ServiceManager::new(LaunchdServiceManager::from_builder(
        builder,
    )?));
}
