use windows_service::{
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager as WindowsServiceManager, ServiceManagerAccess},
};

use crate::{
    service_config::ServiceConfig,
    service_manager::{Result, ServiceManager},
    service_status::ServiceStatus,
};

pub struct Manager {
    service_manager: WindowsServiceManager,
    config: ServiceConfig,
}
impl ServiceManager for Manager {
    fn new(config: ServiceConfig) -> Result<Self> {
        let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
        let service_manager =
            WindowsServiceManager::local_computer(None::<&str>, manager_access).unwrap();
        Ok(Self {
            service_manager,
            config,
        })
    }

    fn install(&self) -> Result<()> {
        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
        if self
            .service_manager
            .open_service(&self.config.name, service_access)
            .is_err()
        {
            let service_info = ServiceInfo {
                name: (&self.config.name).into(),
                display_name: (&self.config.display_name).into(),
                service_type: ServiceType::OWN_PROCESS,
                start_type: ServiceStartType::OnDemand,
                error_control: ServiceErrorControl::Normal,
                executable_path: (&self.config.program).into(),
                launch_arguments: self.config.args_iter().map(Into::into).collect(),
                dependencies: vec![],
                account_name: None, // run as System
                account_password: None,
            };
            let service = self
                .service_manager
                .create_service(
                    &service_info,
                    ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
                )
                .unwrap();
            service.set_description(&self.config.description).unwrap();
        }
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
        let service = self
            .service_manager
            .open_service(&self.config.name, service_access)
            .unwrap();
        service.delete().unwrap();
        Ok(())
    }

    fn start(&self) -> Result<()> {
        let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::START;
        let service = self
            .service_manager
            .open_service(&self.config.name, service_access)
            .unwrap();
        service.start::<String>(&[]).unwrap();
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::STOP
            | ServiceAccess::DELETE
            | ServiceAccess::START;
        let service = self
            .service_manager
            .open_service(&self.config.name, service_access)
            .unwrap();
        let _ = service.stop();
        Ok(())
    }

    fn query_status(&self) -> Result<ServiceStatus> {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::STOP
            | ServiceAccess::DELETE
            | ServiceAccess::START;
        let service = match self
            .service_manager
            .open_service(&self.config.name, service_access)
        {
            Ok(service) => service,
            Err(_) => return Ok(ServiceStatus::NotInstalled),
        };
        match service.query_status().unwrap().current_state {
            windows_service::service::ServiceState::Stopped
            | windows_service::service::ServiceState::StartPending => Ok(ServiceStatus::Stopped),
            _ => Ok(ServiceStatus::Started),
        }
    }
}
