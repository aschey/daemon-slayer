use crate::{service_config::ServiceConfig, service_status::ServiceStatus};

pub trait ServiceManager {
    fn new(config: ServiceConfig) -> Self;
    fn install(&self);
    fn uninstall(&self);
    fn start(&self);
    fn stop(&self);
    fn query_status(&self) -> ServiceStatus;
}
