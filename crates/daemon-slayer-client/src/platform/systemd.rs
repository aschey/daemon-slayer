use crate::{config::Builder, Info, Manager, State};
use std::io;
use systemd_client::{
    create_unit_configuration_file, create_user_unit_configuration_file,
    delete_unit_configuration_file, delete_user_unit_configuration_file,
    manager::{self, SystemdManagerProxyBlocking},
    service, unit, InstallConfiguration, NotifyAccess, ServiceConfiguration, ServiceType,
    ServiceUnitConfiguration, UnitActiveStateType, UnitConfiguration, UnitFileState,
    UnitLoadStateType, UnitSubStateType,
};

#[derive(Clone)]
pub struct SystemdServiceManager {
    config: Builder,
    client: SystemdManagerProxyBlocking<'static>,
}

impl SystemdServiceManager {
    pub(crate) fn from_builder(builder: Builder) -> std::result::Result<Self, io::Error> {
        let client = if builder.is_user() {
            manager::build_blocking_user_proxy().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!("Error connecting to systemd user proxy: {e:?}"),
                )
            })
        } else {
            manager::build_blocking_proxy().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!("Error connecting to systemd proxy: {e:?}"),
                )
            })
        }?;
        Ok(Self {
            config: builder,
            client,
        })
    }

    fn service_file_name(&self) -> String {
        format!("{}.service", self.name())
    }

    fn set_autostart_enabled(&mut self, enabled: bool) -> Result<(), io::Error> {
        self.config.autostart = enabled;
        self.update_autostart()?;
        Ok(())
    }

    fn update_autostart(&self) -> Result<(), io::Error> {
        if self.config.autostart {
            self.client
                .enable_unit_files(&[&self.service_file_name()], false, true)
                .map_err(|e| {
                    io_error(format!(
                        "Error enabling systemd unit file {}: {e:?}",
                        self.service_file_name()
                    ))
                })?;
        } else {
            self.client
                .disable_unit_files(&[&self.service_file_name()], false)
                .map_err(|e| {
                    io_error(format!(
                        "Error disabling systemd unit file {}: {e:?}",
                        self.service_file_name()
                    ))
                })?;
        }
        Ok(())
    }
}

impl Manager for SystemdServiceManager {
    fn on_config_changed(&mut self) -> Result<(), io::Error> {
        let snapshot = self.config.user_config.snapshot();
        self.config.user_config.reload();
        let current = self.config.user_config.load();
        if current.environment_variables != snapshot.environment_variables {
            self.reload_config()?;
        }
        Ok(())
    }

    fn reload_config(&self) -> Result<(), io::Error> {
        let current_state = self.info()?.state;
        self.stop()?;
        self.install()?;
        if current_state == State::Started {
            self.start()?;
        }
        Ok(())
    }

    fn install(&self) -> Result<(), io::Error> {
        let mut unit_config = UnitConfiguration::builder().description(&self.config.description);
        for after in &self.config.systemd_config.after {
            unit_config = unit_config.after(after);
        }

        let mut service_config = ServiceConfiguration::builder()
            .exec_start(
                self.config
                    .full_arguments_iter()
                    .map(String::as_ref)
                    .collect(),
            )
            .ty(ServiceType::Notify)
            .notify_access(NotifyAccess::Main);

        let vars = self.config.environment_variables();
        for (key, value) in &vars {
            service_config = service_config.env(key, value);
        }

        let mut svc_unit_builder = ServiceUnitConfiguration::builder()
            .unit(unit_config)
            .service(service_config);

        if self.config.is_user() {
            svc_unit_builder = svc_unit_builder
                .install(InstallConfiguration::builder().wanted_by("default.target"));
        }

        let svc_unit_literal = format!("{}", svc_unit_builder.build());

        if self.config.is_user() {
            create_user_unit_configuration_file(
                &self.service_file_name(),
                svc_unit_literal.as_bytes(),
            )
        } else {
            create_unit_configuration_file(&self.service_file_name(), svc_unit_literal.as_bytes())
        }
        .map_err(|e| {
            io_error(format!(
                "Error creating unit config file {}: {e:?}",
                self.service_file_name()
            ))
        })?;

        self.update_autostart()?;

        Ok(())
    }

    fn uninstall(&self) -> Result<(), io::Error> {
        if self.config.is_user() {
            delete_user_unit_configuration_file(&self.service_file_name())
        } else {
            delete_unit_configuration_file(&self.service_file_name())
        }
        .map_err(|e| {
            io_error(format!(
                "Error removing systemd config file {:?}: {e:?}",
                &self.service_file_name()
            ))
        })?;
        Ok(())
    }

    fn start(&self) -> Result<(), io::Error> {
        self.client
            .start_unit(&self.service_file_name(), "replace")
            .map_err(|e| {
                io_error(format!(
                    "Error starting systemd unit {}: {e:?}",
                    self.service_file_name()
                ))
            })?;
        Ok(())
    }

    fn stop(&self) -> Result<(), io::Error> {
        if self.info()?.state == State::Started {
            self.client
                .stop_unit(&self.service_file_name(), "replace")
                .map_err(|e| {
                    io_error(format!(
                        "Error stopping systemd unit {}: {e:?}",
                        self.service_file_name()
                    ))
                })?;
        }

        Ok(())
    }

    fn restart(&self) -> Result<(), io::Error> {
        if self.info()?.state == State::Started {
            self.client
                .restart_unit(&self.service_file_name(), "replace")
                .map_err(|e| {
                    io_error(format!(
                        "Error restarting systemd unit {}: {e:?}",
                        self.service_file_name()
                    ))
                })?;
        } else {
            self.start()?;
        }
        Ok(())
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
        self.client
            .reload()
            .map_err(|e| io_error(format!("Error reloading systemd units: {e:?}")))?;

        self.client
            .reset_failed()
            .map_err(|e| io_error(format!("Error resetting failed unit state: {e:?}")))?;

        let svc_unit_path = self
            .client
            .load_unit(&self.service_file_name())
            .map_err(|e| {
                io_error(format!(
                    "Error loading systemd unit {}: {e:?}",
                    self.service_file_name()
                ))
            })?;

        let unit_client = if self.config.is_user() {
            unit::build_blocking_user_proxy(svc_unit_path.clone())
        } else {
            unit::build_blocking_proxy(svc_unit_path.clone())
        }
        .map_err(|e| {
            io_error(format!(
                "Error creating unit client {}: {e:?}",
                svc_unit_path.as_str()
            ))
        })?;

        let unit_props = unit_client
            .get_properties()
            .map_err(|e| io_error(format!("Error getting unit properties: {e:?}")))?;

        let state = match (
            unit_props.load_state,
            unit_props.active_state,
            unit_props.sub_state,
        ) {
            (UnitLoadStateType::Loaded, UnitActiveStateType::Active, UnitSubStateType::Running) => {
                State::Started
            }
            (UnitLoadStateType::NotFound, _, _) => State::NotInstalled,
            _ => State::Stopped,
        };

        let service_client = if self.config.is_user() {
            service::build_blocking_user_proxy(svc_unit_path)
        } else {
            service::build_blocking_proxy(svc_unit_path)
        }
        .map_err(|e| io_error(format!("Error creating unit proxy: {e:?}")))?;

        let service_props = service_client
            .get_properties()
            .map_err(|e| io_error(format!("Error getting service properties: {e:?}")))?;

        let autostart = match (&state, unit_props.unit_file_state) {
            (State::NotInstalled, _) => None,
            (_, UnitFileState::Enabled | UnitFileState::EnabledRuntime | UnitFileState::Static) => {
                Some(true)
            }
            _ => Some(false),
        };

        let pid = if state == State::Started {
            Some(service_props.exec_main_pid)
        } else {
            None
        };

        let last_exit_code = if state == State::NotInstalled {
            None
        } else {
            Some(service_props.exec_main_status)
        };

        Ok(Info {
            pid,
            state,
            autostart,
            last_exit_code,
        })
    }

    fn display_name(&self) -> &str {
        self.config.display_name()
    }

    fn name(&self) -> String {
        self.config.label.application.clone()
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
