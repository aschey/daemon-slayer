use std::{error::Error, result};

use crate::{service_config::ServiceConfig, service_status::ServiceStatus};

pub type Result<T> = result::Result<T, Box<dyn Error>>;
pub trait ServiceManager {
    fn new(config: ServiceConfig) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn install(&self) -> Result<()>;
    fn uninstall(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn query_status(&self) -> Result<ServiceStatus>;
}
