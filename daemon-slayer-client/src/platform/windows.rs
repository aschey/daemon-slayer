use eyre::Context;
use regex::Regex;
use windows_service::{
    service::{
        Service, ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    },
    service_manager::{
        ListServiceType, ServiceActiveState, ServiceEntry, ServiceManager as WindowsServiceManager,
        ServiceManagerAccess,
    },
};

use crate::{Builder, Level, Manager, Result, Status};

pub struct ServiceManager {
    config: Builder,
}

impl ServiceManager {
    fn get_service_status(&self, service: &str, service_type: ServiceType) -> Result<Status> {
        if self
            .find_service(Regex::new(&format!("^{}$", service)).unwrap(), service_type)?
            .is_none()
        {
            return Ok(Status::NotInstalled);
        }

        let service = self.open_service(service)?;
        match service
            .query_status()
            .wrap_err("Error getting service status")?
            .current_state
        {
            ServiceState::Stopped | ServiceState::StartPending => Ok(Status::Stopped),
            _ => Ok(Status::Started),
        }
    }

    fn find_service(&self, re: Regex, service_type: ServiceType) -> Result<Option<ServiceEntry>> {
        let manager = self.get_manager()?;
        let user_service = manager
            .get_all_services(ListServiceType::WIN32, ServiceActiveState::ALL)
            .wrap_err("Error getting list of services")?
            .into_iter()
            .find(|service| {
                service.status.service_type.contains(service_type) && re.is_match(&service.name)
            });
        Ok(user_service)
    }

    fn current_service_name(&self) -> Result<Option<String>> {
        let service = match &self.config.service_level {
            Level::System => self.config.name.clone(),
            Level::User => {
                // User services have a random id appended to the end like this: some_service_name_18dcf87g
                // The id changes every login so we have to search for it
                let re = Regex::new(&format!(r"^{}_[a-z\d]{{8}}$", self.config.name)).unwrap();
                let user_service = self.find_service(re, ServiceType::USER_OWN_PROCESS)?;

                match user_service {
                    Some(service) => service.name,
                    None => return Ok(None),
                }
            }
        };

        Ok(Some(service))
    }

    fn open_service(&self, service: &str) -> Result<Service> {
        let service = self
            .get_manager()?
            .open_service(service, ServiceAccess::all())
            .wrap_err("Error opening service")?;
        Ok(service)
    }

    fn open_current_service(&self) -> Result<Service> {
        let name = match self.current_service_name()? {
            Some(name) => name,
            None => return Err("Unable to find service")?,
        };
        self.open_service(&name)
    }

    fn delete_service(&self, service: &str, service_type: ServiceType) -> Result<()> {
        if self.get_service_status(service, service_type)? != Status::NotInstalled {
            let service = self.open_service(service)?;
            service.delete().wrap_err("Error deleting service")?;
        }
        Ok(())
    }

    fn get_manager(&self) -> Result<WindowsServiceManager> {
        let service_manager =
            WindowsServiceManager::local_computer(None::<&str>, ServiceManagerAccess::all())
                .wrap_err("Error creating service manager")?;
        Ok(service_manager)
    }
}

impl Manager for ServiceManager {
    fn builder(name: impl Into<String>) -> Builder {
        Builder::new(name)
    }

    fn new(name: impl Into<String>) -> Result<Self> {
        Builder::new(name).build()
    }

    fn from_builder(builder: Builder) -> Result<Self> {
        Ok(Self { config: builder })
    }

    fn display_name(&self) -> &str {
        &self.config.display_name
    }

    fn install(&self) -> Result<()> {
        if self.open_service(&self.config.name).is_err() {
            let service_info = ServiceInfo {
                name: (&self.config.name).into(),
                display_name: (&self.config.display_name).into(),
                service_type: match self.config.service_level {
                    Level::System => ServiceType::OWN_PROCESS,
                    Level::User => ServiceType::USER_OWN_PROCESS,
                },
                start_type: ServiceStartType::OnDemand,
                error_control: ServiceErrorControl::Normal,
                executable_path: (&self.config.program).into(),
                launch_arguments: self.config.args_iter().map(Into::into).collect(),
                dependencies: vec![],
                account_name: None, // run as System
                account_password: None,
            };
            let service = self
                .get_manager()?
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
        if self.config.service_level == Level::User {
            let current_service_name = match self.current_service_name()? {
                Some(name) => name,
                None => return Ok(()),
            };
            self.delete_service(&current_service_name, ServiceType::USER_OWN_PROCESS)?;
        }
        let name = self.config.name.clone();
        self.delete_service(&name, ServiceType::OWN_PROCESS)?;

        Ok(())
    }

    fn start(&self) -> Result<()> {
        if self.query_status()? == Status::Started {
            return Ok(());
        }

        let service = self.open_current_service()?;
        service
            .start::<String>(&[])
            .wrap_err("Error starting service")?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if self.query_status()? != Status::Started {
            return Ok(());
        }
        let service = self.open_current_service()?;
        let _ = service.stop().wrap_err("Error stopping service")?;
        Ok(())
    }

    fn query_status(&self) -> Result<Status> {
        let service = match self.current_service_name()? {
            Some(service) => service,
            None => return Ok(Status::NotInstalled),
        };

        if self.config.service_level == Level::User {
            self.get_service_status(&service, ServiceType::USER_OWN_PROCESS)
        } else {
            self.get_service_status(&service, ServiceType::OWN_PROCESS)
        }
    }

    fn args(&self) -> &Vec<String> {
        &self.config.args
    }

    fn description(&self) -> &str {
        &self.config.description
    }
}
