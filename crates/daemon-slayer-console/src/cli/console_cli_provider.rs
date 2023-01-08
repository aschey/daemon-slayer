use crate::Console;
use daemon_slayer_core::{
    cli::{clap, ActionType, CommandConfig, CommandMatch, CommandOutput, CommandType},
    BoxedError,
};

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
                    children: vec![],
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
    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![&self.command]
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        match matched_command
            .as_ref()
            .map(|c| &c.matched_command.command_type)
        {
            Some(CommandType::Subcommand {
                name,
                help_text: _,
                hide: _,
                children: _,
            }) if name == "console" => {
                self.console.run().await.unwrap();
                Ok(CommandOutput::handled(None))
            }
            _ => Ok(CommandOutput::unhandled()),
        }
    }
}
