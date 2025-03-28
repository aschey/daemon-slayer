use async_trait::async_trait;
use daemon_slayer_core::BoxedError;
use daemon_slayer_core::cli::clap::{self, Args, FromArgMatches, Subcommand};
use daemon_slayer_core::cli::{ActionType, CommandMatch, CommandOutput, CommandProvider};

use crate::ProcessManager;

#[derive(Subcommand, Clone, Debug)]
enum ProcessSubcommands {
    /// Show process info
    Info,
    /// Force kill the service process
    Kill,
}

#[derive(Args, Clone, Debug)]
struct ProcessArgs {
    #[command(subcommand)]
    commands: ProcessSubcommands,
}

#[derive(Subcommand)]
enum CliCommands {
    // View and control the service process
    Process(ProcessArgs),
}

#[derive(Clone, Debug)]
pub struct ProcessCliProvider {
    pid: Option<u32>,
    matched_args: Option<ProcessArgs>,
}

impl ProcessCliProvider {
    pub fn new(pid: Option<u32>) -> Self {
        Self {
            pid,
            matched_args: None,
        }
    }
}

#[async_trait]
impl CommandProvider for ProcessCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliCommands::augment_subcommands(command)
    }

    fn matches(&mut self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let command = CliCommands::from_arg_matches(matches).ok()?;
        let CliCommands::Process(args) = command;
        self.matched_args = Some(args);
        Some(CommandMatch {
            action_type: ActionType::Client,
            action: None,
        })
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        let pid = self.pid.as_ref();
        let Some(args) = self.matched_args else {
            return Ok(CommandOutput::unhandled());
        };
        let Some(pid) = pid else {
            return Ok(CommandOutput::handled(
                "Error: process is not running".to_owned(),
            ));
        };
        return Ok(match args.commands {
            ProcessSubcommands::Info => {
                let message = match ProcessManager::new(*pid).process_info() {
                    Some(info) => info.pretty_print(),
                    None => "Process not found".to_owned(),
                };
                CommandOutput::handled(message)
            }
            ProcessSubcommands::Kill => {
                let message = match ProcessManager::kill(*pid) {
                    Some(true) => "Kill signal sent",
                    Some(false) => "Failed to send kill signal",
                    None => "Process not found",
                };
                CommandOutput::handled(message.to_owned())
            }
        });
    }
}
