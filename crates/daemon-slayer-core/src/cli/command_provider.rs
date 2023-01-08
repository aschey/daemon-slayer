use super::{Action, ActionType, ArgMatchesExt, CommandExt, CommandOutput, CommandType};
use crate::{AsAny, BoxedError};

#[derive(Clone, Debug)]
pub struct CommandConfig {
    pub action_type: ActionType,
    pub command_type: CommandType,
    pub action: Option<Action>,
}

#[async_trait::async_trait]
pub trait CommandProvider: AsAny + Send + 'static {
    fn get_commands(&self) -> Vec<&CommandConfig>;

    async fn handle_input(
        self: Box<Self>,
        matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError>;

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<(), BoxedError> {
        Ok(())
    }
}

impl dyn CommandProvider {
    pub fn update_command(&self, mut command: clap::Command) -> clap::Command {
        for command_config in self.get_commands() {
            command = command.add_command_handler(&command_config.command_type);
        }
        command
    }

    pub fn action_type(&self, matches: &clap::ArgMatches) -> ActionType {
        for command_config in self.get_commands() {
            if matches.matches(&command_config.command_type).is_some() {
                return command_config.action_type.clone();
            }
        }

        ActionType::Unknown
    }
}

pub struct CommandMatch {
    pub matched_command: CommandConfig,
    pub matches: clap::ArgMatches,
}
