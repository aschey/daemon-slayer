use async_trait::async_trait;
use daemon_slayer_core::BoxedError;
use daemon_slayer_core::cli::clap::{self, FromArgMatches, Subcommand};
use daemon_slayer_core::cli::{ActionType, CommandMatch, CommandOutput, CommandProvider};

use crate::Console;

#[derive(Subcommand)]
enum CliCommands {
    /// View service console
    Console,
}

pub struct ConsoleCliProvider {
    console: Console,
    matched: bool,
}

impl ConsoleCliProvider {
    pub fn new(console: Console) -> Self {
        Self {
            console,
            matched: false,
        }
    }
}
#[async_trait]
impl CommandProvider for ConsoleCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliCommands::augment_subcommands(command)
    }

    fn matches(&mut self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        CliCommands::from_arg_matches(matches).ok()?;
        self.matched = true;
        Some(CommandMatch {
            action_type: ActionType::Client,
            action: None,
        })
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        if self.matched {
            self.console.run().await?;
            Ok(CommandOutput::handled(None))
        } else {
            Ok(CommandOutput::unhandled())
        }
    }
}
