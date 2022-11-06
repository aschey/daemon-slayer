use std::collections::HashMap;

use super::{Action, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState};

#[async_trait::async_trait]
pub trait CommandProvider: Send {
    async fn handle_input(self: Box<Self>, matches: &clap::ArgMatches) -> InputState;

    fn get_action_type(&self) -> ActionType;

    fn get_commands(&self) -> Vec<&CommandType>;

    fn set_base_commands(&mut self, _: HashMap<Action, CommandType>) {}

    fn initialize(&mut self, _: &clap::ArgMatches) {}

    fn update_command(&self, mut command: clap::Command) -> clap::Command {
        for command_type in self.get_commands() {
            command = command.add_command_handler(command_type);
        }
        command
    }

    fn action_type(&self, matches: &clap::ArgMatches) -> ActionType {
        for command_type in self.get_commands() {
            if matches.matches(command_type) {
                return self.get_action_type();
            }
        }

        ActionType::Unknown
    }
}
