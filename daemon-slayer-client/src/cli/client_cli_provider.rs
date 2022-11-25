use daemon_slayer_core::cli::{
    clap, Action, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState,
};
use std::collections::HashMap;

use crate::{Manager, ServiceManager};

pub struct ClientCliProvider {
    commands: HashMap<Action, CommandType>,
    config: CommandType,
    manager: ServiceManager,
}

impl ClientCliProvider {
    pub fn new(manager: ServiceManager) -> Self {
        Self {
            commands: Default::default(),
            manager,
            config: CommandType::Arg {
                id: "env_var".to_owned(),
                short: Some('e'),
                long: Some("env".to_owned()),
                help_text: Some("set env var".to_string()),
                hide: false,
            },
        }
    }

    pub fn with_action(
        mut self,
        action: Action,
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

    fn set_base_commands(&mut self, commands: HashMap<Action, CommandType>) {
        self.commands = commands;
    }

    fn get_commands(&self) -> Vec<&CommandType> {
        vec![&self.config]
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &daemon_slayer_core::cli::clap::ArgMatches,
    ) -> daemon_slayer_core::cli::InputState {
        for (name, command_type) in &self.commands {
            if name.action_type() == ActionType::Client && matches.matches(command_type) {
                match name {
                    Action::Install => self.manager.install().unwrap(),
                    Action::Uninstall => self.manager.uninstall().unwrap(),
                    Action::Info => {
                        let info = self.manager.info().unwrap();
                        println!("{info:?}");
                    }
                    Action::Start => self.manager.start().unwrap(),
                    Action::Stop => self.manager.stop().unwrap(),
                    Action::Restart => self.manager.restart().unwrap(),
                    Action::Enable => self.manager.set_autostart_enabled(true).unwrap(),
                    Action::Disable => self.manager.set_autostart_enabled(false).unwrap(),
                    Action::Pid => {
                        let pid = self.manager.info().unwrap().pid;
                        println!("{pid:?}");
                    }
                    _ => {}
                }
                return InputState::Handled;
            }
        }
        InputState::Unhandled
    }
}
