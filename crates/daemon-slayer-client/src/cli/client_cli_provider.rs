use crate::Manager;
use daemon_slayer_core::{
    cli::{clap, Action, ActionType, CommandConfig, CommandMatch, CommandType, InputState},
    BoxedError,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct ClientCliProvider {
    commands: HashMap<Action, CommandConfig>,
    manager: Box<dyn Manager>,
}

impl ClientCliProvider {
    pub fn new(manager: Box<dyn Manager>) -> Self {
        let mut commands = HashMap::default();
        commands.insert(
            Action::Install,
            CommandConfig {
                action: Some(Action::Install),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Install.to_string(),
                    help_text: "Install the service using the system's service manager".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );
        commands.insert(
            Action::Uninstall,
            CommandConfig {
                action: Some(Action::Uninstall),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Uninstall.to_string(),
                    help_text: "Uninstall the service from the system's service manager".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );
        commands.insert(
            Action::Start,
            CommandConfig {
                action: Some(Action::Start),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Start.to_string(),
                    help_text: "Start the service".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );
        commands.insert(
            Action::Info,
            CommandConfig {
                action: Some(Action::Info),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Info.to_string(),
                    help_text: "Get the service's current status".into(),
                    hide: false,
                    children: vec![],
                },
            },
        );
        commands.insert(
            Action::Pid,
            CommandConfig {
                action: Some(Action::Pid),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Pid.to_string(),
                    help_text: "Get the service's current PID".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );
        commands.insert(
            Action::Stop,
            CommandConfig {
                action: Some(Action::Stop),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Stop.to_string(),
                    help_text: "Stop the service".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );

        commands.insert(
            Action::Restart,
            CommandConfig {
                action: Some(Action::Restart),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Restart.to_string(),
                    help_text: "Restart the service".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );

        commands.insert(
            Action::Reload,
            CommandConfig {
                action: Some(Action::Reload),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Reload.to_string(),
                    help_text: "Reload the service config".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );

        commands.insert(
            Action::Enable,
            CommandConfig {
                action: Some(Action::Enable),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Enable.to_string(),
                    help_text: "Enable autostart".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );

        commands.insert(
            Action::Disable,
            CommandConfig {
                action: Some(Action::Disable),
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: Action::Disable.to_string(),
                    help_text: "Disable autostart".to_owned(),
                    hide: false,
                    children: vec![],
                },
            },
        );

        Self { commands, manager }
    }

    pub fn with_action(mut self, action: Action, command_type: CommandType) -> Self {
        if let Some(command_config) = self.commands.get_mut(&action) {
            command_config.command_type = command_type;
        }
        self
    }

    pub fn without_action(mut self, action: Action) -> Self {
        self.commands.remove(&action);
        self
    }
}
#[async_trait::async_trait]
impl daemon_slayer_core::cli::CommandProvider for ClientCliProvider {
    fn get_action_type(&self) -> ActionType {
        ActionType::Client
    }

    fn get_commands(&self) -> Vec<&CommandConfig> {
        self.commands.values().collect()
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<InputState, BoxedError> {
        if let Some(CommandMatch {
            matched_command, ..
        }) = matched_command
        {
            if matched_command.action_type == ActionType::Client {
                match matched_command.action {
                    Some(Action::Install) => self.manager.install()?,
                    Some(Action::Uninstall) => self.manager.uninstall()?,
                    Some(Action::Info) => {
                        let info = self.manager.info()?;
                        println!("{}", info.pretty_print());
                    }
                    Some(Action::Start) => self.manager.start()?,
                    Some(Action::Stop) => self.manager.stop()?,
                    Some(Action::Restart) => self.manager.restart()?,
                    Some(Action::Reload) => self.manager.reload_config()?,
                    Some(Action::Enable) => self.manager.enable_autostart()?,
                    Some(Action::Disable) => self.manager.disable_autostart()?,
                    Some(Action::Pid) => {
                        let pid = self.manager.info()?.pid;
                        println!(
                            "{}",
                            pid.map(|p| p.to_string())
                                .unwrap_or_else(|| "Not running".to_owned())
                        );
                    }
                    _ => return Ok(InputState::Unhandled),
                }
                return Ok(InputState::Handled);
            }
        }

        Ok(InputState::Unhandled)
    }
}
