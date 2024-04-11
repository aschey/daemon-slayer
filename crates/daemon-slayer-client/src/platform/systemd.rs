use std::io;

use async_trait::async_trait;
use daemon_slayer_core::socket_activation::SocketType;
use daemon_slayer_core::Label;
use systemd_client::manager::{self, SystemdManagerProxy};
use systemd_client::service::SystemdServiceProxy;
use systemd_client::{
    create_unit_configuration_file, create_user_unit_configuration_file,
    delete_unit_configuration_file, delete_user_unit_configuration_file, service, unit,
    InstallConfiguration, NotifyAccess, OwnedObjectPath, ServiceConfiguration, ServiceType,
    ServiceUnitConfiguration, SocketConfiguration, SystemdUnitProxy, UnitActiveStateType,
    UnitConfiguration, UnitFileState, UnitLoadStateType, UnitProps, UnitSubStateType,
};

use crate::config::systemd::SocketActivationBehavior;
use crate::config::{Builder, Config};
use crate::{Command, Manager, State, Status};

macro_rules! systemd_run {
    ($self:ident, $run_mode:expr, $err_msg:expr, $f:expr) => {
        if !$self.config.activation_socket_config.is_empty()
            && ($run_mode == RunMode::Socket || $run_mode == RunMode::Both)
        {
            #[allow(clippy::redundant_closure_call)]
            $f(
                $self.socket_file_name.as_str(),
                &[$self.socket_file_name.as_str()],
            )
            .await
            .map_err(|e| io_error(format!("{}: {e:?}", $err_msg)))?;
        }

        if !$self.config.has_sockets()
            || $run_mode == RunMode::Service
            || $run_mode == RunMode::Both
        {
            #[allow(clippy::redundant_closure_call)]
            $f(
                $self.service_file_name.as_str(),
                &[$self.service_file_name.as_str()],
            )
            .await
            .map_err(|e| io_error(format!("{}: {e:?}", $err_msg)))?;
        }
    };
}

#[derive(Clone, Debug)]
pub struct SystemdServiceManager {
    config: Builder,
    client: SystemdManagerProxy<'static>,
    service_file_name: String,
    socket_file_name: String,
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
        let service_file_name = format!("{}.service", builder.label.application);
        let socket_file_name = format!("{}.socket", builder.label.application);
        Ok(Self {
            config: builder,
            client,
            service_file_name,
            socket_file_name,
        })
    }

    fn is_enable_all(&self) -> bool {
        self.config.systemd_config.socket_activation_behavior == SocketActivationBehavior::EnableAll
    }

    async fn set_autostart_enabled(&mut self, enabled: bool) -> io::Result<()> {
        self.config.autostart = enabled;
        self.update_autostart().await?;
        Ok(())
    }

    async fn update_autostart(&self) -> io::Result<()> {
        if self.config.autostart {
            systemd_run!(
                self,
                if self.is_enable_all() {
                    RunMode::Both
                } else {
                    RunMode::Socket
                },
                "Error enabling systemd unit file",
                |_, files| self.client.enable_unit_files(files, false, true)
            );
        } else {
            systemd_run!(
                self,
                if self.is_enable_all() {
                    RunMode::Both
                } else {
                    RunMode::Socket
                },
                "Error disabling systemd unit file",
                |_, files| self.client.disable_unit_files(files, false)
            );
        }
        Ok(())
    }

    async fn get_unit_path(&self, name: &str) -> io::Result<OwnedObjectPath> {
        self.client.load_unit(name).await.map_err(|e| {
            io_error(format!(
                "Error loading systemd unit {}: {e:?}",
                self.service_file_name
            ))
        })
    }

    async fn get_unit_client(
        &self,
        svc_unit_path: OwnedObjectPath,
    ) -> io::Result<SystemdUnitProxy> {
        if self.config.is_user() {
            unit::build_nonblock_user_proxy(svc_unit_path.clone()).await
        } else {
            unit::build_nonblock_proxy(svc_unit_path.clone()).await
        }
        .map_err(|e| {
            io_error(format!(
                "Error creating unit client {}: {e:?}",
                svc_unit_path.as_str()
            ))
        })
    }

    async fn get_service_client(
        &self,
        svc_unit_path: OwnedObjectPath,
    ) -> io::Result<SystemdServiceProxy> {
        if self.config.is_user() {
            service::build_nonblock_user_proxy(svc_unit_path).await
        } else {
            service::build_nonblock_proxy(svc_unit_path).await
        }
        .map_err(|e| io_error(format!("Error creating unit proxy: {e:?}")))
    }

    async fn get_socket_state(&self) -> io::Result<Option<UnitProps>> {
        if !self.config.has_sockets() {
            return Ok(None);
        }
        let socket_unit_path = self.get_unit_path(&self.socket_file_name).await?;
        let socket_unit_client = self.get_unit_client(socket_unit_path).await?;
        socket_unit_client
            .get_properties()
            .await
            .map_err(|e| io_error(format!("Error getting unit properties: {e:?}")))
            .map(Some)
    }
}

#[derive(PartialEq, Eq)]
enum RunMode {
    Service,
    Socket,
    Both,
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

        let svc_unit_builder = ServiceUnitConfiguration::builder()
            .unit(unit_config)
            .service(service_config);

        // https://unix.stackexchange.com/questions/404667/systemd-service-what-is-multi-user-target
        let mut install_config = InstallConfiguration::builder().wanted_by("multi-user.target");
        if self.config.is_user() {
            // default.target either points to graphical.target or multi-user.target
            install_config = install_config.wanted_by("default.target");
        }

        let svc_unit_literal = svc_unit_builder.install(install_config).build().to_string();

        if self.config.is_user() {
            create_user_unit_configuration_file(
                &self.service_file_name,
                svc_unit_literal.as_bytes(),
            )
        } else {
            create_unit_configuration_file(&self.service_file_name, svc_unit_literal.as_bytes())
        }
        .map_err(|e| {
            io_error(format!(
                "Error creating unit config file {}: {e:?}",
                self.service_file_name
            ))
        })?;

        if !self.config.activation_socket_config.is_empty() {
            let mut socket_builder = SocketConfiguration::builder()
                .install(InstallConfiguration::builder().wanted_by("sockets.target"));
            for socket in &self.config.activation_socket_config {
                match socket.socket_type() {
                    SocketType::Ipc => {
                        socket_builder = socket_builder.listen_stream(socket.addr());
                    }
                    SocketType::Tcp => {
                        socket_builder = socket_builder.listen_stream(socket.addr());
                    }
                    SocketType::Udp => {
                        socket_builder = socket_builder.listen_datagram(socket.addr());
                    }
                }
            }

            let socket_unit_literal = socket_builder.build().to_string();

            if self.config.is_user() {
                create_user_unit_configuration_file(
                    &self.socket_file_name,
                    socket_unit_literal.as_bytes(),
                )
            } else {
                create_unit_configuration_file(
                    &self.socket_file_name,
                    socket_unit_literal.as_bytes(),
                )
            }
            .map_err(|e| {
                io_error(format!(
                    "Error creating unit config file {}: {e:?}",
                    self.socket_file_name
                ))
            })?
        }

        self.update_autostart().await?;

        Ok(())
    }

    async fn uninstall(&self) -> io::Result<()> {
        if self.config.is_user() {
            delete_user_unit_configuration_file(&self.service_file_name).unwrap();
            delete_user_unit_configuration_file(&self.socket_file_name)
        } else {
            delete_unit_configuration_file(&self.service_file_name).unwrap();
            delete_unit_configuration_file(&self.socket_file_name)
        }
        .map_err(|e| {
            io_error(format!(
                "Error removing systemd config file {:?}: {e:?}",
                &self.service_file_name
            ))
        })?;
        Ok(())
    }

    async fn start(&self) -> io::Result<()> {
        systemd_run!(
            self,
            if self.is_enable_all() {
                RunMode::Both
            } else {
                RunMode::Socket
            },
            "Error starting systemd unit",
            |file, _| self.client.start_unit(file, "replace")
        );

        Ok(())
    }

    async fn stop(&self) -> io::Result<()> {
        if matches!(
            self.status().await?.state,
            State::Started | State::Listening
        ) {
            systemd_run!(
                self,
                RunMode::Both,
                "Error stopping systemd unit",
                |file, _| { self.client.stop_unit(file, "replace") }
            );
        }

        Ok(())
    }

    async fn restart(&self) -> io::Result<()> {
        let state = self.status().await?.state;
        match state {
            State::Started => {
                systemd_run!(
                    self,
                    RunMode::Both,
                    "Error restarting systemd unit",
                    |file, _| self.client.restart_unit(file, "replace")
                );
            }
            State::Listening => {
                systemd_run!(
                    self,
                    RunMode::Socket,
                    "Error restarting systemd unit",
                    |file, _| self.client.restart_unit(file, "replace")
                );
            }
            _ => {
                self.start().await?;
            }
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

        let svc_unit_path = self.get_unit_path(&self.service_file_name).await?;

        let unit_client = self.get_unit_client(svc_unit_path.clone()).await?;
        let unit_props = unit_client
            .get_properties()
            .await
            .map_err(|e| io_error(format!("Error getting unit properties: {e:?}")))?;

        let socket_state = self.get_socket_state().await?;
        let state = match (
            unit_props.load_state,
            unit_props.active_state,
            unit_props.sub_state,
        ) {
            (UnitLoadStateType::Loaded, UnitActiveStateType::Active, UnitSubStateType::Running) => {
                State::Started
            }
            (UnitLoadStateType::NotFound, _, _) => State::NotInstalled,
            _ => {
                if let Some(socket_state) = socket_state.clone() {
                    if matches!(
                        (
                            socket_state.load_state,
                            socket_state.active_state,
                            socket_state.sub_state
                        ),
                        (
                            UnitLoadStateType::Loaded,
                            UnitActiveStateType::Active,
                            UnitSubStateType::Listening
                        )
                    ) {
                        State::Listening
                    } else {
                        State::Stopped
                    }
                } else {
                    State::Stopped
                }
            }
        };

        let service_client = self.get_service_client(svc_unit_path).await?;

        let service_props = service_client
            .get_properties()
            .await
            .map_err(|e| io_error(format!("Error getting service properties: {e:?}")))?;

        let autostart = match (&state, unit_props.unit_file_state) {
            (State::NotInstalled, _) => None,
            (_, UnitFileState::Enabled | UnitFileState::EnabledRuntime | UnitFileState::Static) => {
                Some(true)
            }
            _ => {
                if let Some(socket_state) = socket_state {
                    if matches!(
                        socket_state.unit_file_state,
                        UnitFileState::Enabled
                            | UnitFileState::EnabledRuntime
                            | UnitFileState::Static
                    ) {
                        Some(true)
                    } else {
                        Some(false)
                    }
                } else {
                    Some(false)
                }
            }
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
