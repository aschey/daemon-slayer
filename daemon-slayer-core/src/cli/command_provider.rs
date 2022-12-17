use std::{any::Any, collections::HashMap};

use crate::AsAny;

use super::{Action, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState};

#[derive(Clone)]
pub struct CommandConfig {
    pub action_type: ActionType,
    pub command_type: CommandType,
    pub action: Option<Action>,
}

#[async_trait::async_trait]
pub trait CommandProvider: AsAny + Send + 'static {
    async fn handle_input(
        self: Box<Self>,
        matches: &clap::ArgMatches,
        matched_command: &Option<CommandConfig>,
    ) -> InputState;

    fn get_action_type(&self) -> ActionType;

    fn get_commands(&self) -> Vec<&CommandConfig>;

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        _matched_command: &Option<CommandConfig>,
    ) {
    }

    fn update_command(&self, mut command: clap::Command) -> clap::Command {
        for command_config in self.get_commands() {
            command = command.add_command_handler(&command_config.command_type);
        }
        command
    }

    fn action_type(&self, matches: &clap::ArgMatches) -> ActionType {
        for command_config in self.get_commands() {
            if matches.matches(&command_config.command_type) {
                return self.get_action_type();
            }
        }

        ActionType::Unknown
    }
}
