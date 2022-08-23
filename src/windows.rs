use eyre::Context;
use windows_service::{
    service::{
        Service, ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    },
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

impl Manager {
    fn open_service(&self) -> Result<Service> {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::START
            | ServiceAccess::STOP
            | ServiceAccess::CHANGE_CONFIG
            | ServiceAccess::DELETE;

        let service = self
            .service_manager
            .open_service(&self.config.name, service_access)
            .wrap_err("Error opening service")?;

        Ok(service)
    }
}

impl ServiceManager for Manager {
    fn new(config: ServiceConfig) -> Result<Self> {
        let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
        let service_manager = WindowsServiceManager::local_computer(None::<&str>, manager_access)
            .wrap_err("Error creating service manager")?;
        Ok(Self {
            service_manager,
            config,
        })
    }

    fn install(&self) -> Result<()> {
        if self.open_service().is_err() {
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
                .wrap_err("Error creating service")?;
            service
                .set_description(&self.config.description)
                .wrap_err("Error setting description")?;
        }
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let service = self.open_service()?;
        service.delete().wrap_err("Error deleting service")?;
        Ok(())
    }

    fn start(&self) -> Result<()> {
        let service = self.open_service()?;
        service
            .start::<String>(&[])
            .wrap_err("Error starting service")?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if self.query_status()? != ServiceStatus::Started {
            return Ok(());
        }
        let service = self.open_service()?;
        let _ = service.stop().wrap_err("Error stopping service")?;
        Ok(())
    }

    fn query_status(&self) -> Result<ServiceStatus> {
        let service = match self.open_service() {
            Ok(service) => service,
            Err(_) => return Ok(ServiceStatus::NotInstalled),
        };
        match service
            .query_status()
            .wrap_err("Error getting service status")?
            .current_state
        {
            ServiceState::Stopped | ServiceState::StartPending => Ok(ServiceStatus::Stopped),
            _ => Ok(ServiceStatus::Started),
        }
    }
}
