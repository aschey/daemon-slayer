use crate::{
    configuration::{windows::Trustee, Builder, Level},
    Info, Manager, State,
};
use regex::Regex;
use registry::{Data, Hive, Security};
use std::io;
use std::{thread, time::Duration};
use utfx::U16CString;
use windows_service::{
    service::{
        Service, ServiceAccess, ServiceErrorControl, ServiceExitCode, ServiceInfo,
        ServiceStartType, ServiceState, ServiceType,
    },
    service_manager::{
        ListServiceType, ServiceActiveState, ServiceEntry, ServiceManager, ServiceManagerAccess,
    },
};

#[derive(Clone)]
enum ServiceAccessMode {
    Read,
    Write,
    Execute,
    ChangeConfig,
}

#[derive(Clone)]
pub struct WindowsServiceManager {
    configuration: Builder,
}

impl WindowsServiceManager {
    pub(crate) fn from_builder(builder: Builder) -> Result<Self, io::Error> {
        Ok(Self {
            configuration: builder,
        })
    }

    fn query_info(&self, service_name: &str, service_type: ServiceType) -> Result<Info, io::Error> {
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

        let service = self.open_service(service_name, ServiceAccessMode::Read)?;

        let service_status = service.query_status().map_err(|e| {
            io_error(format!(
                "Error getting status for service {service_name}: {e:?}"
            ))
        })?;

        let state = match service_status.current_state {
            ServiceState::Stopped | ServiceState::StartPending => State::Stopped,
            _ => State::Started,
        };

        let last_exit_code = match service_status.exit_code {
            ServiceExitCode::Win32(code) => Some(code),
            ServiceExitCode::ServiceSpecific(code) => Some(code),
        };

        let autostart_service = if self.configuration.is_user() {
            self.open_base_service(ServiceAccessMode::Read)?
        } else {
            service
        };

        let autostart = autostart_service
            .query_config()
            .map_err(|e| io_error(format!("Error querying service config: {e:?}")))?
            .start_type
            == ServiceStartType::AutoStart;
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
    ) -> Result<Option<ServiceEntry>, io::Error> {
        let re_text = if service_type == ServiceType::USER_OWN_PROCESS {
            // User services have a random id appended to the end like this: some_service_name_18dcf87g
            // The id changes every login so we have to search for it
            format!(r"^{}_[a-z\d]+$", self.name())
        } else {
            format!("^{}$", self.name())
        };
        let re = Regex::new(&re_text).unwrap();
        let manager = self.get_manager(mode)?;
        let user_service = manager
            .get_all_services(ListServiceType::WIN32, ServiceActiveState::ALL)
            .map_err(|e| io_error(format!("Error getting list of services: {e:?}")))?
            .into_iter()
            .find(|service| {
                service.status.service_type.contains(service_type) && re.is_match(&service.name)
            });
        Ok(user_service)
    }

    fn current_service_name(&self) -> Result<Option<String>, io::Error> {
        let service = match &self.configuration.service_level {
            Level::System => self.name(),
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

    fn open_service(&self, service: &str, mode: ServiceAccessMode) -> Result<Service, io::Error> {
        let service = self
            .get_manager(mode.clone())?
            .open_service(
                service,
                match mode {
                    ServiceAccessMode::Write => ServiceAccess::all(),
                    ServiceAccessMode::ChangeConfig => {
                        ServiceAccess::QUERY_CONFIG
                            | ServiceAccess::QUERY_STATUS
                            | ServiceAccess::CHANGE_CONFIG
                    }
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
            .map_err(|e| io_error(format!("Error opening service {service}: {e:?}")))?;
        Ok(service)
    }

    fn open_current_service(&self, mode: ServiceAccessMode) -> Result<Service, io::Error> {
        let name = match self.current_service_name()? {
            Some(name) => name,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Unable to find current service",
                ));
            }
        };
        self.open_service(&name, mode)
    }

    fn open_base_service(&self, mode: ServiceAccessMode) -> Result<Service, io::Error> {
        self.open_service(&self.name(), mode)
    }

    fn delete_service(
        &self,
        service_name: &str,
        service_type: ServiceType,
    ) -> Result<(), io::Error> {
        // For user-level services, the service won't show up in the service list so we have to
        // attempt to open it to see if it exists
        if self.configuration.service_level == Level::User
            && service_type == ServiceType::OWN_PROCESS
        {
            if let Ok(service) = self.open_service(service_name, ServiceAccessMode::Write) {
                service.delete().map_err(|e| {
                    io_error(format!("Error deleting user service {service_name}: {e:?}"))
                })?;
                return Ok(());
            }
        }
        if self.query_info(service_name, service_type)?.state != State::NotInstalled {
            let service = self.open_service(service_name, ServiceAccessMode::Write)?;
            service.delete().map_err(|e| {
                io_error(format!(
                    "Error deleting system service {service_name}: {e:?}"
                ))
            })?;
        }
        Ok(())
    }

    fn get_manager(&self, mode: ServiceAccessMode) -> Result<ServiceManager, io::Error> {
        let service_manager = ServiceManager::local_computer(
            None::<&str>,
            match mode {
                ServiceAccessMode::Write => ServiceManagerAccess::all(),
                _ => ServiceManagerAccess::CONNECT | ServiceManagerAccess::ENUMERATE_SERVICE,
            },
        )
        .map_err(|e| io_error(format!("Error connecting to local service manager: {e:?}")))?;
        Ok(service_manager)
    }

    fn get_service_info(&self) -> ServiceInfo {
        ServiceInfo {
            name: self.name().into(),
            display_name: self.display_name().into(),
            service_type: match self.configuration.service_level {
                Level::System => ServiceType::OWN_PROCESS,
                Level::User => ServiceType::USER_OWN_PROCESS,
            },
            start_type: if self.configuration.autostart {
                ServiceStartType::AutoStart
            } else {
                ServiceStartType::OnDemand
            },
            error_control: ServiceErrorControl::Normal,
            executable_path: (&self.configuration.program).into(),
            launch_arguments: self
                .configuration
                .arguments_iter()
                .map(Into::into)
                .collect(),
            dependencies: vec![],
            account_name: None, // run as System
            account_password: None,
        }
    }

    fn wait_for_state(&self, desired_state: State) -> Result<(), io::Error> {
        let attempts = 5;
        for _ in 0..attempts {
            if self.info()?.state == desired_state {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("Failed to reach desired state: {desired_state:#?}"),
        ))
    }

    fn set_autostart_enabled(&mut self, enabled: bool) -> Result<(), io::Error> {
        let service = self.open_base_service(ServiceAccessMode::ChangeConfig)?;
        let mut configuration = service
            .query_config()
            .map_err(|e| io_error(format!("Error querying service config: {e:?}")))?;
        configuration.start_type = if enabled {
            ServiceStartType::AutoStart
        } else {
            ServiceStartType::OnDemand
        };
        let exe_string = configuration.executable_path.to_string_lossy().to_string();
        let mut parts = exe_string.split(' ');

        let exe_path = parts.next().unwrap();
        let arguments = parts.collect::<Vec<_>>();
        let info = ServiceInfo {
            name: self.name().into(),
            display_name: self.display_name().into(),
            service_type: configuration.service_type,
            start_type: configuration.start_type,
            error_control: configuration.error_control,
            executable_path: exe_path.into(),
            launch_arguments: arguments.iter().map(Into::into).collect(),
            dependencies: configuration.dependencies,
            account_name: configuration.account_name,
            account_password: None,
        };
        service
            .change_config(&info)
            .map_err(|e| io_error(format!("Error changing service config: {e:?}")))?;
        Ok(())
    }

    fn reg_basekey(&self) -> String {
        format!(r"SYSTEM\CurrentControlSet\Services\{}", self.name())
    }

    fn add_environment_variables(&self) -> Result<(), io::Error> {
        let env_vars = self.configuration.environment_variables();
        if env_vars.is_empty() {
            return Ok(());
        }
        let vars = env_vars
            .iter()
            .filter_map(|(key, value)| U16CString::from_os_str(format!("{key}={value}")).ok())
            .collect::<Vec<_>>();

        let key = Hive::LocalMachine
            .open(self.reg_basekey(), Security::Write)
            .map_err(|e| from_registry_key_error(&self.reg_basekey(), e))?;

        let reg_value = "Environment";
        let registry_data = Data::MultiString(vars);

        key.set_value(reg_value, &registry_data).map_err(|e| {
            from_registry_value_error(
                &format!("{reg_value} = {registry_data}"),
                &key.to_string(),
                e,
            )
        })?;
        Ok(())
    }
}

impl Manager for WindowsServiceManager {
    fn display_name(&self) -> &str {
        self.configuration.display_name()
    }

    fn name(&self) -> String {
        self.configuration.label.application.clone()
    }

    fn reload_configuration(&self) -> Result<(), io::Error> {
        let current_state = self.info()?.state;
        self.stop()?;
        self.add_environment_variables()?;
        if current_state == State::Started {
            self.start()?;
        }
        Ok(())
    }

    fn on_configuration_changed(&mut self) -> Result<(), io::Error> {
        let snapshot = self.configuration.user_configuration.snapshot();
        self.configuration.user_configuration.reload();
        let current = self.configuration.user_configuration.load();
        if current.environment_variables != snapshot.environment_variables {
            self.reload_configuration()?;
        }
        Ok(())
    }

    fn install(&self) -> Result<(), io::Error> {
        if self.open_base_service(ServiceAccessMode::Write).is_err() {
            let service_info = self.get_service_info();
            let manager = self.get_manager(ServiceAccessMode::Write)?;
            let service = manager
                .create_service(
                    &service_info,
                    ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
                )
                .map_err(|e| {
                    io_error(format!(
                        "Error creating service {:#?}: {e:?}",
                        service_info.name
                    ))
                })?;

            service
                .set_description(&self.configuration.description)
                .map_err(|e| {
                    io_error(format!(
                        "Error setting service description to \"{}\": {e:?}",
                        self.configuration.description
                    ))
                })?;

            if let Some((trustee, access)) =
                &self.configuration.windows_configuration.additional_access
            {
                let trustee = match trustee {
                    Trustee::CurrentUser => windows_service::service::Trustee::CurrentUser,
                    Trustee::Name(name) => windows_service::service::Trustee::Name(name.clone()),
                };

                let mut service_access = ServiceAccess::empty();
                for permission in access.iter() {
                    service_access |= match permission {
                        crate::configuration::windows::ServiceAccess::QueryStatus => {
                            ServiceAccess::QUERY_STATUS
                        }
                        crate::configuration::windows::ServiceAccess::Start => ServiceAccess::START,
                        crate::configuration::windows::ServiceAccess::Stop => ServiceAccess::STOP,
                        crate::configuration::windows::ServiceAccess::PauseContinue => {
                            ServiceAccess::PAUSE_CONTINUE
                        }
                        crate::configuration::windows::ServiceAccess::Interrogate => {
                            ServiceAccess::INTERROGATE
                        }
                        crate::configuration::windows::ServiceAccess::Delete => {
                            ServiceAccess::DELETE
                        }
                        crate::configuration::windows::ServiceAccess::QueryConfig => {
                            ServiceAccess::QUERY_CONFIG
                        }
                        crate::configuration::windows::ServiceAccess::ChangeConfig => {
                            ServiceAccess::CHANGE_CONFIG
                        }
                    }
                }
                let service_access_ = service_access.clone();
                service
                    .grant_user_access(trustee, service_access)
                    .map_err(|e| {
                        io_error(format!(
                            "Error granting user access: {service_access_:?}: {e:?}"
                        ))
                    })?;
            }
        }
        self.add_environment_variables()?;
        Ok(())
    }

    fn uninstall(&self) -> Result<(), io::Error> {
        if self.configuration.is_user() {
            if let Some(current_service_name) = self.current_service_name()? {
                self.delete_service(&current_service_name, ServiceType::USER_OWN_PROCESS)?;
            };
            // Still attempt to delete the service template if the user service wasn't found
            // Could be that the user service wasn't created yet
        }
        let name = self.name();
        self.delete_service(&name, ServiceType::OWN_PROCESS)?;

        Ok(())
    }

    fn start(&self) -> Result<(), io::Error> {
        if self.info()?.state == State::Started {
            return Ok(());
        }

        let service = self.open_current_service(ServiceAccessMode::Execute)?;
        service
            .start::<String>(&[])
            .map_err(|e| io_error(format!("Error starting service: {e:?}")))?;
        Ok(())
    }

    fn stop(&self) -> Result<(), io::Error> {
        if self.info()?.state != State::Started {
            return Ok(());
        }
        let service = self.open_current_service(ServiceAccessMode::Execute)?;
        service
            .stop()
            .map_err(|e| io_error(format!("Error stopping service: {e:?}")))?;
        Ok(())
    }

    fn restart(&self) -> Result<(), io::Error> {
        if self.info()?.state == State::Started {
            self.stop()?;
            self.wait_for_state(State::Stopped)?;
        }
        self.start()?;
        self.wait_for_state(State::Started)
    }

    fn enable_autostart(&mut self) -> Result<(), io::Error> {
        self.set_autostart_enabled(true)?;
        Ok(())
    }

    fn disable_autostart(&mut self) -> Result<(), io::Error> {
        self.set_autostart_enabled(false)?;
        Ok(())
    }

    fn info(&self) -> Result<Info, io::Error> {
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

        if self.configuration.is_user() {
            self.query_info(&service, ServiceType::USER_OWN_PROCESS)
        } else {
            self.query_info(&service, ServiceType::OWN_PROCESS)
        }
    }

    fn arguments(&self) -> &Vec<String> {
        &self.configuration.arguments
    }

    fn description(&self) -> &str {
        &self.configuration.description
    }
}

fn io_error(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, message)
}

fn from_registry_key_error(path: &str, err: registry::key::Error) -> io::Error {
    match err {
        registry::key::Error::NotFound(_, err) => io::Error::new(
            io::ErrorKind::NotFound,
            format!("Registry path {path} was not found: {err:?}"),
        ),
        registry::key::Error::PermissionDenied(_, err) => io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Permission denied opening registry path {path}: {err:?}"),
        ),
        registry::key::Error::Unknown(_, err) => {
            io::Error::new(err.kind(), format!("Error opening registry path {path}"))
        }
        registry::key::Error::InvalidNul(err) => io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Registry key contains an invalid null character: {err:?}"),
        ),
        err => io_error(format!(
            "Unknown error opening registry path {path}: {err:?}"
        )),
    }
}

fn from_registry_value_error(value: &str, path: &str, err: registry::value::Error) -> io::Error {
    match err {
        registry::value::Error::NotFound(_, err) => io::Error::new(
            io::ErrorKind::NotFound,
            format!("Registry value {value} in path {path} was not found: {err:?}"),
        ),
        registry::value::Error::PermissionDenied(_, _) => io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Permission denied for registry value {value} in path {path}"),
        ),
        registry::value::Error::UnhandledType(reg_type) => io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unhandled type {reg_type} for registry value {value} in path {path}"),
        ),
        registry::value::Error::InvalidNul(err) => io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid null byte for registry value {value} in path {path}: {err:?}"),
        ),
        registry::value::Error::MissingNul(err) => io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Missing null termination byte for registry value {value} in path {path}: {err:?}"
            ),
        ),
        registry::value::Error::MissingMultiNul => io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Missing null termination bytes for registry value {value} in path {path}: {err:?}"
            ),
        ),
        registry::value::Error::InvalidUtf16(err) => io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid UTF-16 daata for registry value {value} in path {path}: {err:?}"),
        ),
        registry::value::Error::Unknown(_, err) => io::Error::new(
            err.kind(),
            format!("Unknown error for registry value {value} in path {path}: {err:?}"),
        ),
        err => io::Error::new(
            io::ErrorKind::Other,
            format!("Unknown error for registry value {value} in path {path}: {err:?}"),
        ),
    }
}
