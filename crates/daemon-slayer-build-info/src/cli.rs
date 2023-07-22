use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, Args, FromArgMatches},
        ActionType, CommandMatch, CommandOutput, CommandProvider,
    },
    BoxedError,
};
use vergen_pretty::Pretty;

#[derive(Args)]
struct CliArgs {
    #[arg(long)]
    build_info: bool,
}

pub struct BuildInfoCliProvider {
    output: Pretty,
    matched_command: Option<CliArgs>,
}

impl BuildInfoCliProvider {
    pub fn new(output: Pretty) -> Self {
        Self {
            output,
            matched_command: None,
        }
    }
}

#[async_trait]
impl CommandProvider for BuildInfoCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliArgs::augment_args(command)
    }

    fn matches(&mut self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let cli_args = CliArgs::from_arg_matches(matches).ok()?;
        if !cli_args.build_info || matches.subcommand().is_some() {
            return None;
        }
        self.matched_command = Some(cli_args);
        Some(CommandMatch {
            action_type: ActionType::Other,
            action: None,
        })
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        match self.matched_command {
            Some(CliArgs { build_info: true }) => {
                let mut buf = Vec::new();
                self.output.display(&mut buf).unwrap();

                Ok(CommandOutput::handled(String::from_utf8(buf).unwrap()))
            }
            _ => Ok(CommandOutput::unhandled()),
        }
    }
}
