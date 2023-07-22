use std::{collections::HashMap, io};

use bollard::{
    container::{
        CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
        UpdateContainerOptions,
    },
    service::{ContainerState, HostConfig, RestartPolicy, RestartPolicyNameEnum},
    Docker,
};
use daemon_slayer_core::{async_trait, Label};

use crate::{
    config::{Builder, Config},
    Info, Manager, State,
};

#[derive(Debug, Clone)]
pub struct DockerServiceManager {
    config: Builder,
    docker: Docker,
}

impl DockerServiceManager {
    pub(crate) async fn from_builder(config: Builder) -> std::result::Result<Self, io::Error> {
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

    async fn reload_config(&mut self) -> Result<(), io::Error> {
        let current_state = self.info().await?.state;
        self.config.user_config.reload();
        self.uninstall().await?;
        self.install().await?;
        if current_state == State::Started {
            self.start().await?;
        }
        Ok(())
    }

    async fn on_config_changed(&mut self) -> Result<(), io::Error> {
        let snapshot = self.config.user_config.snapshot();
        self.config.user_config.reload();
        let current = self.config.user_config.load();
        if current.environment_variables != snapshot.environment_variables {
            self.reload_config().await?;
        }
        Ok(())
    }

    async fn install(&self) -> Result<(), io::Error> {
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

    async fn uninstall(&self) -> Result<(), io::Error> {
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

    async fn start(&self) -> Result<(), io::Error> {
        self.docker
            .start_container::<&str>(&self.name(), None)
            .await
            .unwrap();

        Ok(())
    }

    async fn stop(&self) -> Result<(), io::Error> {
        self.docker
            .stop_container(&self.name(), None)
            .await
            .unwrap();
        Ok(())
    }

    async fn restart(&self) -> Result<(), io::Error> {
        self.docker
            .restart_container(&self.name(), None)
            .await
            .unwrap();
        Ok(())
    }

    async fn enable_autostart(&mut self) -> Result<(), io::Error> {
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

    async fn disable_autostart(&mut self) -> Result<(), io::Error> {
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

    async fn info(&self) -> Result<Info, io::Error> {
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

            let info = Info {
                label: self.config.label.clone(),
                state,
                autostart,
                pid: container_state.pid.map(|p| p as u32),
                id: inspect.id.map(|id| id[0..12].to_owned()),
                last_exit_code: container_state.exit_code.map(|e| e as i32),
            };

            return Ok(info);
        }
        let info = Info {
            label: self.config.label.clone(),
            state: State::NotInstalled,
            autostart: None,
            pid: None,
            id: None,
            last_exit_code: None,
        };

        Ok(info)
    }
}
