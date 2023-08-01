use crate::{
    config::{Builder, Config},
    Command, Manager, State, Status,
};
use daemon_slayer_core::{async_trait, Label};
use std::io;
use systemd_client::{
    create_unit_configuration_file, create_user_unit_configuration_file,
    delete_unit_configuration_file, delete_user_unit_configuration_file,
    manager::{self, SystemdManagerProxy},
    service, unit, InstallConfiguration, NotifyAccess, ServiceConfiguration, ServiceType,
    ServiceUnitConfiguration, UnitActiveStateType, UnitConfiguration, UnitFileState,
    UnitLoadStateType, UnitSubStateType,
};

#[derive(Clone, Debug)]
pub struct SystemdServiceManager {
    config: Builder,
    client: SystemdManagerProxy<'static>,
}

impl SystemdServiceManager {
    pub(crate) async fn from_builder(builder: Builder) -> io::Result<Self> {
        let client = if builder.is_user() {
            manager::build_nonblock_user_proxy().await.map_err(|e| {
                io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!("Error connecting to systemd user proxy: {e:?}"),
                )
            })
        } else {
            manager::build_nonblock_proxy().await.map_err(|e| {
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

    async fn set_autostart_enabled(&mut self, enabled: bool) -> io::Result<()> {
        self.config.autostart = enabled;
        self.update_autostart().await?;
        Ok(())
    }

    async fn update_autostart(&self) -> io::Result<()> {
        if self.config.autostart {
            self.client
                .enable_unit_files(&[&self.service_file_name()], false, true)
                .await
                .map_err(|e| {
                    io_error(format!(
                        "Error enabling systemd unit file {}: {e:?}",
                        self.service_file_name()
                    ))
                })?;
        } else {
            self.client
                .disable_unit_files(&[&self.service_file_name()], false)
                .await
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

#[async_trait]
impl Manager for SystemdServiceManager {
    async fn on_config_changed(&mut self) -> io::Result<()> {
        let snapshot = self.config.user_config.snapshot();
        self.config.user_config.reload();
        let current = self.config.user_config.load();
        if current.environment_variables != snapshot.environment_variables {
            self.reload_config().await?;
        }
        Ok(())
    }

    async fn reload_config(&mut self) -> io::Result<()> {
        let current_state = self.status().await?.state;
        self.config.user_config.reload();
        self.stop().await?;
        self.install().await?;
        self.client
            .reload()
            .await
            .map_err(|e| io_error(format!("Error reloading systemd units: {e:?}")))?;
        if current_state == State::Started {
            self.start().await?;
        }
        Ok(())
    }

    async fn install(&self) -> io::Result<()> {
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

        self.update_autostart().await?;

        Ok(())
    }

    async fn uninstall(&self) -> io::Result<()> {
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

    async fn start(&self) -> io::Result<()> {
        self.client
            .start_unit(&self.service_file_name(), "replace")
            .await
            .map_err(|e| {
                io_error(format!(
                    "Error starting systemd unit {}: {e:?}",
                    self.service_file_name()
                ))
            })?;
        Ok(())
    }

    async fn stop(&self) -> io::Result<()> {
        if self.status().await?.state == State::Started {
            self.client
                .stop_unit(&self.service_file_name(), "replace")
                .await
                .map_err(|e| {
                    io_error(format!(
                        "Error stopping systemd unit {}: {e:?}",
                        self.service_file_name()
                    ))
                })?;
        }

        Ok(())
    }

    async fn restart(&self) -> io::Result<()> {
        if self.status().await?.state == State::Started {
            self.client
                .restart_unit(&self.service_file_name(), "replace")
                .await
                .map_err(|e| {
                    io_error(format!(
                        "Error restarting systemd unit {}: {e:?}",
                        self.service_file_name()
                    ))
                })?;
        } else {
            self.start().await?;
        }
        Ok(())
    }

    async fn enable_autostart(&mut self) -> io::Result<()> {
        self.set_autostart_enabled(true).await?;
        Ok(())
    }

    async fn disable_autostart(&mut self) -> io::Result<()> {
        self.set_autostart_enabled(false).await?;
        Ok(())
    }

    async fn status(&self) -> io::Result<Status> {
        self.client
            .reload()
            .await
            .map_err(|e| io_error(format!("Error reloading systemd units: {e:?}")))?;

        self.client
            .reset_failed()
            .await
            .map_err(|e| io_error(format!("Error resetting failed unit state: {e:?}")))?;

        let svc_unit_path = self
            .client
            .load_unit(&self.service_file_name())
            .await
            .map_err(|e| {
                io_error(format!(
                    "Error loading systemd unit {}: {e:?}",
                    self.service_file_name()
                ))
            })?;

        let unit_client = if self.config.is_user() {
            unit::build_nonblock_user_proxy(svc_unit_path.clone()).await
        } else {
            unit::build_nonblock_proxy(svc_unit_path.clone()).await
        }
        .map_err(|e| {
            io_error(format!(
                "Error creating unit client {}: {e:?}",
                svc_unit_path.as_str()
            ))
        })?;

        let unit_props = unit_client
            .get_properties()
            .await
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
            service::build_nonblock_user_proxy(svc_unit_path).await
        } else {
            service::build_nonblock_proxy(svc_unit_path).await
        }
        .map_err(|e| io_error(format!("Error creating unit proxy: {e:?}")))?;

        let service_props = service_client
            .get_properties()
            .await
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

        Ok(Status {
            pid,
            state,
            autostart,
            last_exit_code,
            id: None,
        })
    }

    async fn status_command(&self) -> io::Result<Command> {
        let service = format!("{}.service", self.config.label.application);
        if self.config.is_user() {
            Ok(Command {
                program: "systemctl".to_owned(),
                args: vec!["status".to_owned(), "--user".to_owned(), service],
            })
        } else {
            Ok(Command {
                program: "systemctl".to_owned(),
                args: vec!["status".to_owned(), service],
            })
        }
    }

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
