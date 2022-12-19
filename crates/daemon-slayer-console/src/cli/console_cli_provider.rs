use crate::Console;
use daemon_slayer_core::cli::{clap, ActionType, CommandConfig, CommandType, InputState};
use std::error::Error;

pub struct ConsoleCliProvider {
    command: CommandConfig,
    console: Console,
}

impl ConsoleCliProvider {
    pub fn new(console: Console) -> Self {
        Self {
            console,
            command: CommandConfig {
                action_type: ActionType::Client,
                action: None,
                command_type: CommandType::Subcommand {
                    name: "console".to_owned(),
                    help_text: "view service console".to_owned(),
                    hide: false,
                    children: None,
                },
            },
        }
    }
    pub fn with_action(mut self, command_type: CommandType) -> Self {
        self.command.command_type = command_type;
        self
    }
}
#[async_trait::async_trait]
impl daemon_slayer_core::cli::CommandProvider for ConsoleCliProvider {
    fn get_action_type(&self) -> ActionType {
        ActionType::Client
    }

    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![&self.command]
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandConfig>,
    ) -> Result<InputState, Box<dyn Error>> {
        match matched_command.as_ref().map(|c| &c.command_type) {
            Some(CommandType::Subcommand {
                name,
                help_text: _,
                hide: _,
                children: _,
            }) if name == "console" => {
                self.console.run().await.unwrap();
                Ok(InputState::Handled)
            }
            _ => Ok(InputState::Unhandled),
        }
    }
}
