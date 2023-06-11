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
}

impl<H: daemon_slayer_core::health_check::HealthCheck + Clone + Send> HealthCheckCliProvider<H> {
    pub fn new(health_check: H) -> Self {
        Self { health_check }
    }
}

#[async_trait]
impl<H: HealthCheck + Clone + Send + 'static> CommandProvider for HealthCheckCliProvider<H> {
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
        if let Ok(CliCommands::Health) = CliCommands::from_arg_matches(matches) {
            Ok(match self.health_check.invoke().await {
                Ok(()) => CommandOutput::handled("Healthy".to_owned()),
                Err(e) => CommandOutput::handled(format!("Unhealthy: {e:?}")),
            })
        } else {
            Ok(CommandOutput::unhandled())
        }
    }
}
