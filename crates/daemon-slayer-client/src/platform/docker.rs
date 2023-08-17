use std::collections::HashMap;
use std::io;

use bollard::container::{
    CreateContainerOptions, ListContainersOptions, RemoveContainerOptions, UpdateContainerOptions,
};
use bollard::service::{ContainerState, HostConfig, RestartPolicy, RestartPolicyNameEnum};
use bollard::Docker;
use daemon_slayer_core::{async_trait, Label};

use crate::config::{Builder, Config};
use crate::{Command, Manager, State, Status};

#[derive(Debug, Clone)]
pub struct DockerServiceManager {
    config: Builder,
    docker: Docker,
}

impl DockerServiceManager {
    pub(crate) async fn from_builder(config: Builder) -> io::Result<Self> {
        let docker = Docker::connect_with_local_defaults().unwrap();
        docker
            .ping()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e))?;
        Ok(Self { config, docker })
    }
}

#[async_trait]
impl Manager for DockerServiceManager {
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

    async fn status_command(&self) -> io::Result<Command> {
        Ok(Command {
            program: "docker".to_owned(),
            args: vec![
                "ps".to_owned(),
                "-f".to_owned(),
                format!("name={}", self.config.label.application),
            ],
        })
    }

    async fn reload_config(&mut self) -> io::Result<()> {
        let current_state = self.status().await?.state;
        self.config.user_config.reload();
        self.uninstall().await?;
        self.install().await?;
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
        let mut config = bollard::container::Config {
            image: Some(self.config.program.name().to_owned()),
            env: Some(
                self.config
                    .user_config
                    .load()
                    .environment_variables
                    .iter()
                    .map(|e| format!("{}={}", e.name, e.value))
                    .collect(),
            ),
            ..Default::default()
        };
        if let Some(configure) = &self.config.configure_container {
            configure(&mut config);
        }

        self.docker
            .create_container::<&str, String>(
                Some(CreateContainerOptions {
                    name: &self.name(),
                    ..Default::default()
                }),
                config,
            )
            .await
            .unwrap();
        Ok(())
    }

    async fn uninstall(&self) -> io::Result<()> {
        self.stop().await.unwrap();
        self.docker
            .remove_container(
                &self.name(),
                Some(RemoveContainerOptions {
                    v: true,
                    force: false,
                    link: false,
                }),
            )
            .await
            .unwrap();
        Ok(())
    }

    async fn start(&self) -> io::Result<()> {
        self.docker
            .start_container::<&str>(&self.name(), None)
            .await
            .unwrap();

        Ok(())
    }

    async fn stop(&self) -> io::Result<()> {
        self.docker
            .stop_container(&self.name(), None)
            .await
            .unwrap();
        Ok(())
    }

    async fn restart(&self) -> io::Result<()> {
        self.docker
            .restart_container(&self.name(), None)
            .await
            .unwrap();
        Ok(())
    }

    async fn enable_autostart(&mut self) -> io::Result<()> {
        self.docker
            .update_container::<&str>(
                &self.name(),
                UpdateContainerOptions {
                    restart_policy: Some(RestartPolicy {
                        name: Some(RestartPolicyNameEnum::ALWAYS),
                        maximum_retry_count: None,
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        Ok(())
    }

    async fn disable_autostart(&mut self) -> io::Result<()> {
        self.docker
            .update_container::<&str>(
                &self.name(),
                UpdateContainerOptions {
                    restart_policy: Some(RestartPolicy {
                        name: Some(RestartPolicyNameEnum::NO),
                        maximum_retry_count: None,
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        Ok(())
    }

    async fn status(&self) -> io::Result<Status> {
        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions::<&str> {
                all: true,
                filters: HashMap::from([("name", vec![self.name().as_str()])]),
                ..Default::default()
            }))
            .await
            .unwrap();
        if !containers.is_empty() {
            let inspect = self
                .docker
                .inspect_container(&self.name(), None)
                .await
                .unwrap();
            let state = match inspect.state {
                Some(ContainerState {
                    running: Some(true),
                    paused: Some(false) | None,
                    ..
                }) => State::Started,
                _ => State::Stopped,
            };
            let inspect = self
                .docker
                .inspect_container(&self.name(), None)
                .await
                .unwrap();
            let container_state = inspect.state.unwrap();

            let autostart = if matches!(
                inspect.host_config,
                Some(HostConfig {
                    restart_policy: Some(RestartPolicy {
                        name: Some(
                            RestartPolicyNameEnum::ALWAYS
                                | RestartPolicyNameEnum::ON_FAILURE
                                | RestartPolicyNameEnum::UNLESS_STOPPED,
                        ),
                        ..
                    }),
                    ..
                })
            ) {
                Some(true)
            } else {
                Some(false)
            };

            let info = Status {
                state,
                autostart,
                pid: container_state.pid.map(|p| p as u32),
                id: inspect.id.map(|id| id[0..12].to_owned()),
                last_exit_code: container_state.exit_code.map(|e| e as i32),
            };

            return Ok(info);
        }
        let info = Status {
            state: State::NotInstalled,
            autostart: None,
            pid: None,
            id: None,
            last_exit_code: None,
        };

        Ok(info)
    }
}
