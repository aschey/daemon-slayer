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
}

impl BuildInfoCliProvider {
    pub fn new(output: Pretty) -> Self {
        Self { output }
    }
}

#[async_trait]
impl CommandProvider for BuildInfoCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliArgs::augment_args(command)
    }

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        CliArgs::from_arg_matches(matches).ok()?;
        Some(CommandMatch {
            action_type: ActionType::Other,
            action: None,
        })
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        match CliArgs::from_arg_matches(matches) {
            Ok(_) => {
                let mut buf = Vec::new();
                self.output.display(&mut buf).unwrap();

                Ok(CommandOutput::handled(String::from_utf8(buf).unwrap()))
            }
            Err(_) => Ok(CommandOutput::unhandled()),
        }
    }
}
