use daemon_slayer_core::cli::{
    clap, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState,
};
use std::{collections::HashMap, hash::Hash, marker::PhantomData};
use strum_macros::{Display, EnumString};

use crate::{Manager, ServiceManager};

#[derive(Display, Clone, PartialEq, Eq, Hash, Debug, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum ClientAction {
    Install,
    Uninstall,
    Info,
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
    Pid,
}

pub struct ClientCliProvider {
    commands: HashMap<ClientAction, CommandType>,
    manager: ServiceManager,
}

impl ClientCliProvider {
    pub fn new(manager: ServiceManager) -> Self {
        let mut commands = HashMap::default();
        commands.insert(
            ClientAction::Install,
            CommandType::Subcommand {
                name: ClientAction::Install.to_string(),
                help_text: "Install the service using the system's service manager".to_owned(),
                hide: false,
            },
        );
        commands.insert(
            ClientAction::Uninstall,
            CommandType::Subcommand {
                name: ClientAction::Uninstall.to_string(),
                help_text: "Uninstall the service from the system's service manager".to_owned(),
                hide: false,
            },
        );
        commands.insert(
            ClientAction::Start,
            CommandType::Subcommand {
                name: ClientAction::Start.to_string(),
                help_text: "Start the service".to_owned(),
                hide: false,
            },
        );
        commands.insert(
            ClientAction::Info,
            CommandType::Subcommand {
                name: ClientAction::Info.to_string(),
                help_text: "Get the service's current status".into(),
                hide: false,
            },
        );
        commands.insert(
            ClientAction::Pid,
            CommandType::Subcommand {
                name: ClientAction::Pid.to_string(),
                help_text: "Get the service's current PID".to_owned(),
                hide: false,
            },
        );
        commands.insert(
            ClientAction::Stop,
            CommandType::Subcommand {
                name: ClientAction::Stop.to_string(),
                help_text: "Stop the service".to_owned(),
                hide: false,
            },
        );

        commands.insert(
            ClientAction::Restart,
            CommandType::Subcommand {
                name: ClientAction::Restart.to_string(),
                help_text: "Restart the service".to_owned(),
                hide: false,
            },
        );

        commands.insert(
            ClientAction::Enable,
            CommandType::Subcommand {
                name: ClientAction::Enable.to_string(),
                help_text: "Enable autostart".to_owned(),
                hide: false,
            },
        );

        commands.insert(
            ClientAction::Disable,
            CommandType::Subcommand {
                name: ClientAction::Disable.to_string(),
                help_text: "Disable autostart".to_owned(),
                hide: false,
            },
        );

        Self { commands, manager }
    }
    pub fn with_action(
        mut self,
        action: ClientAction,
        command_type: impl Into<Option<CommandType>>,
    ) -> Self {
        if let Some(command_type) = command_type.into() {
            self.commands.insert(action, command_type);
        } else {
            self.commands.remove(&action);
        }

        self
    }
}
#[async_trait::async_trait]
impl daemon_slayer_core::cli::CommandProvider for ClientCliProvider {
    fn get_action_type(&self) -> ActionType {
        ActionType::Client
    }

    fn get_commands(&self) -> Vec<&CommandType> {
        self.commands.values().collect()
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &daemon_slayer_core::cli::clap::ArgMatches,
    ) -> daemon_slayer_core::cli::InputState {
        for (name, command_type) in &self.commands {
            if matches.matches(command_type) {
                match name {
                    ClientAction::Install => self.manager.install().unwrap(),
                    ClientAction::Uninstall => self.manager.uninstall().unwrap(),
                    ClientAction::Info => {
                        let info = self.manager.info().unwrap();
                        println!("{info:?}");
                    }
                    ClientAction::Start => self.manager.start().unwrap(),
                    ClientAction::Stop => self.manager.stop().unwrap(),
                    ClientAction::Restart => self.manager.restart().unwrap(),
                    ClientAction::Enable => self.manager.set_autostart_enabled(true).unwrap(),
                    ClientAction::Disable => self.manager.set_autostart_enabled(false).unwrap(),
                    ClientAction::Pid => {
                        let pid = self.manager.info().unwrap().pid;
                        println!("{pid:?}");
                    }
                }
                return InputState::Handled;
            }
        }
        InputState::Unhandled
    }
}
