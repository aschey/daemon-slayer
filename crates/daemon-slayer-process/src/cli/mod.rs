use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, Args, FromArgMatches, Subcommand},
        ActionType, CommandMatch, CommandOutput, CommandProvider,
    },
    BoxedError,
};

use crate::ProcessManager;

#[derive(Subcommand)]
enum ProcessSubcommands {
    /// Show process info
    Info,
    /// Force kill the service process
    Kill,
}

#[derive(Args)]
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
}

impl ProcessCliProvider {
    pub fn new(pid: Option<u32>) -> Self {
        Self { pid }
    }
}

#[async_trait]
impl CommandProvider for ProcessCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliCommands::augment_subcommands(command)
    }

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        CliCommands::from_arg_matches(matches).ok()?;
        Some(CommandMatch {
            action_type: ActionType::Client,
            action: None,
        })
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        let pid = self.pid.as_ref();
        let Ok(CliCommands::Process(args)) = CliCommands::from_arg_matches(matches) else {
            return Ok(CommandOutput::unhandled());
        };
        let Some(pid) = pid else {
            return Ok(CommandOutput::handled("Error: process is not running".to_owned()));
        };
        return Ok(match args.commands {
            ProcessSubcommands::Info => CommandOutput::handled(
                ProcessManager::new(*pid)
                    .process_info()
                    // This shouldn't happen since we have a pid
                    .expect("Failed to load process info")
                    .pretty_print(),
            ),
            ProcessSubcommands::Kill => {
                ProcessManager::kill(*pid);
                CommandOutput::handled("Process killed".to_owned())
            }
        });
    }
}
