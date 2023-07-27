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
mod docker;
use daemon_slayer_core::Label;
use docker::*;

use crate::{
    config::{Builder, Program, ServiceType},
    ServiceManager,
};

pub fn builder(label: Label, program: Program) -> Builder {
    Builder::new(label, program)
}

pub(crate) async fn get_manager(builder: Builder) -> io::Result<ServiceManager> {
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
