use std::io;
use std::time::Duration;

use async_trait::async_trait;
use daemon_slayer_core::Label;
use regex::Regex;
use registry::{Data, Hive, Security};
use utfx::U16CString;
use windows_service::service::{
    Service, ServiceAccess, ServiceErrorControl, ServiceExitCode, ServiceInfo, ServiceStartType,
    ServiceState, ServiceType,
};
use windows_service::service_manager::{
    ListServiceType, ServiceActiveState, ServiceEntry, ServiceManager, ServiceManagerAccess,
};

use crate::config::windows::Trustee;
use crate::config::{Builder, Config, Level};
use crate::{Command, Manager, State, Status};

#[derive(Clone)]
enum ServiceAccessMode {
    Read,
    Write,
    Execute,
    ChangeConfig,
}

#[derive(Clone, Debug)]
pub struct WindowsServiceManager {
    config: Builder,
}

impl WindowsServiceManager {
    pub(crate) fn from_builder(builder: Builder) -> io::Result<Self> {
        Ok(Self { config: builder })
    }

    async fn query_info(
        &self,
        service_name: &str,
        service_type: ServiceType,
    ) -> io::Result<Status> {
        if self
            .find_service(service_type, ServiceAccessMode::Read)?
            .is_none()
        {
            return Ok(Status {
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                id: None,
                last_exit_code: None,
            });
        }

        // Service might've been uninstalled as we were querying it
        let Ok(service) = self.open_service(service_name, ServiceAccessMode::Read) else {
            return Ok(Status {
                state: State::NotInstalled,
                autostart: None,
                pid: None,
                id: None,
                last_exit_code: None,
            });
        };

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

        let autostart_service = if self.config.is_user() {
            self.open_base_service(ServiceAccessMode::Read)?
        } else {
            service
        };

        let autostart = autostart_service
            .query_config()
            .map_err(|e| io_error(format!("Error querying service config: {e:?}")))?
            .start_type
            == ServiceStartType::AutoStart;
        Ok(Status {
            state,
            autostart: Some(autostart),
            pid: service_status.process_id,
            id: None,
            last_exit_code: last_exit_code.map(|code| code as i32),
        })
    }

    fn find_service(
        &self,
        service_type: ServiceType,
        mode: ServiceAccessMode,
    ) -> io::Result<Option<ServiceEntry>> {
        let re_text = if service_type == ServiceType::USER_OWN_PROCESS {
            // User services have a random id called a LUID appended to the end like this:
            // some_service_name_18dcf87g. The id changes every login so we have to
            // search for it. There does not seem to be any API we can use to get the
            // LUID in a cleaner way.
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

    fn current_service_name(&self) -> io::Result<Option<String>> {
        let service = match &self.config.service_level {
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

    fn open_service(&self, service: &str, mode: ServiceAccessMode) -> io::Result<Service> {
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

    fn open_current_service(&self, mode: ServiceAccessMode) -> io::Result<Service> {
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

    fn open_base_service(&self, mode: ServiceAccessMode) -> io::Result<Service> {
        self.open_service(&self.name(), mode)
    }

    async fn delete_service(
        &self,
        service_name: &str,
        service_type: ServiceType,
    ) -> io::Result<()> {
        // For user-level services, the service won't show up in the service list so we have to
        // attempt to open it to see if it exists
        if self.config.service_level == Level::User && service_type == ServiceType::OWN_PROCESS {
            if let Ok(service) = self.open_service(service_name, ServiceAccessMode::Write) {
                service.delete().map_err(|e| {
                    io_error(format!("Error deleting user service {service_name}: {e:?}"))
                })?;
                return Ok(());
            }
        }
        if self.query_info(service_name, service_type).await?.state != State::NotInstalled {
            let service = self.open_service(service_name, ServiceAccessMode::Write)?;
            service.delete().map_err(|e| {
                io_error(format!(
                    "Error deleting system service {service_name}: {e:?}"
                ))
            })?;
        }
        Ok(())
    }

    fn get_manager(&self, mode: ServiceAccessMode) -> io::Result<ServiceManager> {
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
            executable_path: self.config.program.full_name().into(),
            launch_arguments: self.config.arguments_iter().map(Into::into).collect(),
            dependencies: vec![],
            account_name: None, // run as System
            account_password: None,
        }
    }

    async fn wait_for_state(&self, desired_state: State) -> io::Result<()> {
        let attempts = 5;
        for _ in 0..attempts {
            if self.status().await?.state == desired_state {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("Failed to reach desired state: {desired_state:#?}"),
        ))
    }

    fn set_autostart_enabled(&mut self, enabled: bool) -> io::Result<()> {
        let service = self.open_base_service(ServiceAccessMode::ChangeConfig)?;
        let mut config = service
            .query_config()
            .map_err(|e| io_error(format!("Error querying service config: {e:?}")))?;
        config.start_type = if enabled {
            ServiceStartType::AutoStart
        } else {
            ServiceStartType::OnDemand
        };
        let full_path = config.executable_path.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Service exe path contains invalid unicode: {:?}",
                    config.executable_path
                ),
            )
        })?;

        // If the path contains spaces, it might be escaped
        // Unescape the whole string here so we don't incorrectly parse the double-escaped text
        let mut full_exe_path_parsed =
            windows_args::Args::parse_cmd(&full_path.replace(r#"\""#, r#"""#))
                .filter(|a| !a.is_empty());

        let exe_path = full_exe_path_parsed.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Service exe path is empty")
        })?;

        let info = ServiceInfo {
            name: self.name().into(),
            display_name: config.display_name,
            service_type: config.service_type,
            start_type: config.start_type,
            error_control: config.error_control,
            executable_path: exe_path.into(),
            launch_arguments: full_exe_path_parsed.map(Into::into).collect(),
            dependencies: config.dependencies,
            account_name: config.account_name,
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

    fn add_environment_variables(&self) -> io::Result<()> {
        let env_vars = self.config.environment_variables();
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

#[async_trait]
impl Manager for WindowsServiceManager {
    fn display_name(&self) -> &str {
        self.config.display_name()
    }

    fn name(&self) -> String {
        self.config.label.application.clone()
    }

    fn label(&self) -> &Label {
        &self.config.label
    }

    fn config(&self) -> Config {
        self.config.clone().into()
    }

    async fn status_command(&self) -> io::Result<Command> {
        let name = self.current_service_name()?;
        let Some(name) = name else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Service not found"));
        };
        Ok(Command {
            program: "sc.exe".to_owned(),
            args: vec!["query".to_owned(), name],
        })
    }

    async fn reload_config(&mut self) -> io::Result<()> {
        let current_state = self.status().await?.state;
        self.config.user_config.reload();
        self.stop().await?;
        self.add_environment_variables()?;
        if current_state == State::Started {
            self.start().await?;
        }
        Ok(())
    }

    async fn on_config_changed(&mut self) -> io::Result<()> {
        let snapshot = self.config.user_config.snapshot();
        self.config.user_config.reload();
        let current = self.config.user_config.load();
        if current.environment_variables != snapshot.environment_variables {
            self.reload_config().await?;
        }
        Ok(())
    }

    async fn install(&self) -> io::Result<()> {
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
                        "Error creating service {:?}: {e:?}",
                        service_info.name
                    ))
                })?;

            service
                .set_description(&self.config.description)
                .map_err(|e| {
                    io_error(format!(
                        "Error setting service description to \"{}\": {e:?}",
                        self.config.description
                    ))
                })?;

            for (trustee, access) in &self.config.windows_config.additional_access {
                let trustee = match trustee {
                    Trustee::CurrentUser => windows_service::service::Trustee::CurrentUser,
                    Trustee::Name(name) => windows_service::service::Trustee::Name(name.clone()),
                };

                let mut service_access = ServiceAccess::empty();
                for permission in access.iter() {
                    service_access |= match permission {
                        crate::config::windows::ServiceAccess::QueryStatus => {
                            ServiceAccess::QUERY_STATUS
                        }
                        crate::config::windows::ServiceAccess::Start => ServiceAccess::START,
                        crate::config::windows::ServiceAccess::Stop => ServiceAccess::STOP,
                        crate::config::windows::ServiceAccess::PauseContinue => {
                            ServiceAccess::PAUSE_CONTINUE
                        }
                        crate::config::windows::ServiceAccess::Interrogate => {
                            ServiceAccess::INTERROGATE
                        }
                        crate::config::windows::ServiceAccess::Delete => ServiceAccess::DELETE,
                        crate::config::windows::ServiceAccess::QueryConfig => {
                            ServiceAccess::QUERY_CONFIG
                        }
                        crate::config::windows::ServiceAccess::ChangeConfig => {
                            ServiceAccess::CHANGE_CONFIG
                        }
                    }
                }

                service
                    .grant_user_access(trustee, service_access)
                    .map_err(|e| {
                        io_error(format!(
                            "Error granting user access: {service_access:?}: {e:?}"
                        ))
                    })?;
            }
        }
        self.add_environment_variables()?;
        Ok(())
    }

    async fn uninstall(&self) -> io::Result<()> {
        if self.status().await?.state == State::Started {
            // Make sure we stop the service before attempting to uninstall, otherwise the uninstall
            // can hang
            self.stop().await?;
            self.wait_for_state(State::Stopped).await?;
        }
        if self.config.is_user() {
            if let Some(current_service_name) = self.current_service_name()? {
                self.delete_service(&current_service_name, ServiceType::USER_OWN_PROCESS)
                    .await?;
            };
            // Still attempt to delete the service template if the user service wasn't found
            // Could be that the user service wasn't created yet
        }
        let name = self.name();
        self.delete_service(&name, ServiceType::OWN_PROCESS).await?;

        Ok(())
    }

    async fn start(&self) -> io::Result<()> {
        if self.status().await?.state == State::Started {
            return Ok(());
        }

        let service = self.open_current_service(ServiceAccessMode::Execute)?;
        service
            .start::<String>(&[])
            .map_err(|e| io_error(format!("Error starting service: {e:?}")))?;
        Ok(())
    }

    async fn stop(&self) -> io::Result<()> {
        if self.status().await?.state != State::Started {
            return Ok(());
        }
        let service = self.open_current_service(ServiceAccessMode::Execute)?;
        service
            .stop()
            .map_err(|e| io_error(format!("Error stopping service: {e:?}")))?;
        Ok(())
    }

    async fn restart(&self) -> io::Result<()> {
        if self.status().await?.state == State::Started {
            self.stop().await?;
            self.wait_for_state(State::Stopped).await?;
        }
        self.start().await?;
        self.wait_for_state(State::Started).await
    }

    async fn enable_autostart(&mut self) -> io::Result<()> {
        self.set_autostart_enabled(true)?;
        Ok(())
    }

    async fn disable_autostart(&mut self) -> io::Result<()> {
        self.set_autostart_enabled(false)?;
        Ok(())
    }

    async fn status(&self) -> io::Result<Status> {
        let service = match self.current_service_name()? {
            Some(service) => service,
            None => {
                return Ok(Status {
                    state: State::NotInstalled,
                    autostart: None,
                    pid: None,
                    id: None,
                    last_exit_code: None,
                });
            }
        };

        if self.config.is_user() {
            self.query_info(&service, ServiceType::USER_OWN_PROCESS)
                .await
        } else {
            self.query_info(&service, ServiceType::OWN_PROCESS).await
        }
    }

    async fn pid(&self) -> io::Result<Option<u32>> {
        let Some(service_name) = self.current_service_name()? else {
            return Ok(None);
        };
        let Ok(service) = self.open_service(&service_name, ServiceAccessMode::Read) else {
            return Ok(None);
        };

        let service_status = service.query_status().map_err(|e| {
            io_error(format!(
                "Error getting status for service {service_name}: {e:?}"
            ))
        })?;
        Ok(service_status.process_id)
    }

    fn arguments(&self) -> &Vec<String> {
        &self.config.arguments
    }

    fn description(&self) -> &str {
        &self.config.description
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
            format!("Invalid UTF-16 data for registry value {value} in path {path}: {err:?}"),
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
