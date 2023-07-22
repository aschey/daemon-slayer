use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, FromArgMatches, Subcommand},
        ActionType, CommandMatch, CommandOutput, CommandProvider,
    },
    health_check::HealthCheck,
    BoxedError,
};

#[derive(Subcommand)]
enum CliCommands {
    Health,
}

#[derive(Clone)]
pub struct HealthCheckCliProvider<H: HealthCheck + Clone + Send> {
    health_check: H,
    matched: bool,
}

impl<H: daemon_slayer_core::health_check::HealthCheck + Clone + Send> HealthCheckCliProvider<H> {
    pub fn new(health_check: H) -> Self {
        Self {
            health_check,
            matched: false,
        }
    }
}

#[async_trait]
impl<H: HealthCheck + Clone + Send + 'static> CommandProvider for HealthCheckCliProvider<H> {
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
            Ok(match self.health_check.invoke().await {
                Ok(()) => CommandOutput::handled("Healthy".to_owned()),
                Err(e) => CommandOutput::handled(format!("Unhealthy: {e:?}")),
            })
        } else {
            Ok(CommandOutput::unhandled())
        }
    }
}
