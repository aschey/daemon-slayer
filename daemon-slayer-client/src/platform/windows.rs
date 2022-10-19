use crate::{config::Builder, Info, Level, Manager, Result, State};
use eyre::Context;
use regex::Regex;
use registry::{Data, Hive, Security};
use std::{thread, time::Duration};
use utfx::U16CString;
use windows_service::{
    service::{
        Service, ServiceAccess, ServiceConfig, ServiceErrorControl, ServiceExitCode, ServiceInfo,
        ServiceStartType, ServiceState, ServiceType,
    },
    service_manager::{
        ListServiceType, ServiceActiveState, ServiceEntry, ServiceManager as WindowsServiceManager,
        ServiceManagerAccess,
    },
};

#[derive(Clone)]
enum ServiceAccessMode {
    Read,
    Write,
    Execute,
}

#[derive(Clone)]
pub struct ServiceManager {
    config: Builder,
}

impl ServiceManager {
    fn query_info(&self, service: &str, service_type: ServiceType) -> Result<Info> {
        if self
            .find_service(service_type, ServiceAccessMode::Read)?
            .is_none()
        {
            return Ok(Info {
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                last_exit_code: None,
            });
        }

        let service = self.open_service(service, ServiceAccessMode::Read)?;

        let service_status = service
            .query_status()
            .wrap_err("Error getting service status")?;

        let state = match service_status.current_state {
            ServiceState::Stopped | ServiceState::StartPending => State::Stopped,
            _ => State::Started,
        };

        let last_exit_code = match service_status.exit_code {
            ServiceExitCode::Win32(code) => Some(code),
            ServiceExitCode::ServiceSpecific(code) => Some(code),
        };

        let autostart_service = if self.config.is_user() {
            self.open_base_service(ServiceAccessMode::Read)?
        } else {
            service
        };

        let autostart = autostart_service.query_config()?.start_type == ServiceStartType::AutoStart;
        Ok(Info {
            state,
            autostart: Some(autostart),
            pid: service_status.process_id,
            last_exit_code: last_exit_code.map(|code| code as i32),
        })
    }

    fn find_service(
        &self,
        service_type: ServiceType,
        mode: ServiceAccessMode,
    ) -> Result<Option<ServiceEntry>> {
        let re_text = if self.config.is_user() {
            // User services have a random id appended to the end like this: some_service_name_18dcf87g
            // The id changes every login so we have to search for it
            format!(r"^{}_[a-z\d]+$", self.config.name)
        } else {
            format!("^{}$", self.config.name)
        };
        let re = Regex::new(&re_text).unwrap();
        let manager = self.get_manager(mode)?;
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
                let user_service =
                    self.find_service(ServiceType::USER_OWN_PROCESS, ServiceAccessMode::Read)?;

                match user_service {
                    Some(service) => service.name,
                    None => return Ok(None),
                }
            }
        };

        Ok(Some(service))
    }

    fn open_service(&self, service: &str, mode: ServiceAccessMode) -> Result<Service> {
        let service = self
            .get_manager(mode.clone())?
            .open_service(
                service,
                match mode {
                    ServiceAccessMode::Write => ServiceAccess::all(),
                    ServiceAccessMode::Read => {
                        ServiceAccess::QUERY_CONFIG | ServiceAccess::QUERY_STATUS
                    }
                    ServiceAccessMode::Execute => {
                        ServiceAccess::QUERY_CONFIG
                            | ServiceAccess::QUERY_STATUS
                            | ServiceAccess::START
                            | ServiceAccess::STOP
                    }
                },
            )
            .wrap_err("Error opening service")?;
        Ok(service)
    }

    fn open_current_service(&self, mode: ServiceAccessMode) -> Result<Service> {
        let name = match self.current_service_name()? {
            Some(name) => name,
            None => return Err("Unable to find service")?,
        };
        self.open_service(&name, mode)
    }

    fn open_base_service(&self, mode: ServiceAccessMode) -> Result<Service> {
        self.open_service(&self.config.name, mode)
    }

    fn delete_service(&self, service: &str, service_type: ServiceType) -> Result<()> {
        if self.query_info(service, service_type)?.state != State::NotInstalled {
            let service = self.open_service(service, ServiceAccessMode::Write)?;
            service.delete().wrap_err("Error deleting service")?;
        }
        Ok(())
    }

    fn get_manager(&self, mode: ServiceAccessMode) -> Result<WindowsServiceManager> {
        let service_manager = WindowsServiceManager::local_computer(
            None::<&str>,
            match mode {
                ServiceAccessMode::Write => ServiceManagerAccess::all(),
                _ => ServiceManagerAccess::CONNECT | ServiceManagerAccess::ENUMERATE_SERVICE,
            },
        )
        .wrap_err("Error creating service manager")?;
        Ok(service_manager)
    }

    fn get_service_info(&self) -> ServiceInfo {
        ServiceInfo {
            name: (&self.config.name).into(),
            display_name: (&self.config.display_name).into(),
            service_type: match self.config.service_level {
                Level::System => ServiceType::OWN_PROCESS,
                Level::User => ServiceType::USER_OWN_PROCESS,
            },
            start_type: if self.config.autostart {
                ServiceStartType::AutoStart
            } else {
                ServiceStartType::OnDemand
            },
            error_control: ServiceErrorControl::Normal,
            executable_path: (&self.config.program).into(),
            launch_arguments: self.config.args_iter().map(Into::into).collect(),
            dependencies: vec![],
            account_name: None, // run as System
            account_password: None,
        }
    }

    fn wait_for_state(&self, desired_state: State) -> Result<()> {
        let attempts = 5;
        for _ in 0..attempts {
            if self.info()?.state == desired_state {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("Failed to stop")?
    }

    fn reg_basekey(&self) -> String {
        format!(r"SYSTEM\CurrentControlSet\Services\{}", self.config.name)
    }

    fn add_environment_variables(&self) -> Result<()> {
        if self.config.env_vars.is_empty() {
            return Ok(());
        }
        let vars = self
            .config
            .env_vars
            .iter()
            .filter_map(|(key, value)| U16CString::from_os_str(format!("{key}={value}")).ok())
            .collect::<Vec<_>>();

        let key = Hive::LocalMachine.open(self.reg_basekey(), Security::Write)?;
        key.set_value("Environment", &Data::MultiString(vars))?;
        Ok(())
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

    fn name(&self) -> &str {
        &self.config.name
    }

    fn install(&self) -> Result<()> {
        if self.open_base_service(ServiceAccessMode::Write).is_err() {
            let service_info = self.get_service_info();
            let service = self
                .get_manager(ServiceAccessMode::Write)?
                .create_service(
                    &service_info,
                    ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
                )
                .wrap_err("Error creating service")?;

            service
                .set_description(&self.config.description)
                .wrap_err("Error setting description")?;
        }
        self.add_environment_variables()?;
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        if self.config.is_user() {
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
        if self.info()?.state == State::Started {
            return Ok(());
        }

        let service = self.open_current_service(ServiceAccessMode::Execute)?;
        service
            .start::<String>(&[])
            .wrap_err("Error starting service")?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if self.info()?.state != State::Started {
            return Ok(());
        }
        let service = self.open_current_service(ServiceAccessMode::Execute)?;
        let _ = service.stop().wrap_err("Error stopping service")?;
        Ok(())
    }

    fn restart(&self) -> Result<()> {
        if self.info()?.state == State::Started {
            self.stop()?;
            self.wait_for_state(State::Stopped)?;
        }
        self.start()?;
        self.wait_for_state(State::Started)
    }

    fn set_autostart_enabled(&mut self, enabled: bool) -> Result<()> {
        let service = self.open_base_service(ServiceAccessMode::Write)?;
        self.config.autostart = enabled;
        service.change_config(&self.get_service_info())?;
        Ok(())
    }

    fn info(&self) -> Result<Info> {
        let service = match self.current_service_name()? {
            Some(service) => service,
            None => {
                return Ok(Info {
                    state: State::NotInstalled,
                    autostart: None,
                    pid: None,
                    last_exit_code: None,
                })
            }
        };

        if self.config.is_user() {
            self.query_info(&service, ServiceType::USER_OWN_PROCESS)
        } else {
            self.query_info(&service, ServiceType::OWN_PROCESS)
        }
    }

    fn args(&self) -> &Vec<String> {
        &self.config.args
    }

    fn description(&self) -> &str {
        &self.config.description
    }
}
