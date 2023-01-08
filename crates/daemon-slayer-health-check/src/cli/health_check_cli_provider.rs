use daemon_slayer_core::{
    cli::{
        clap, ActionType, CommandConfig, CommandMatch, CommandOutput, CommandProvider, CommandType,
    },
    health_check::HealthCheck,
    BoxedError,
};

#[derive(Clone)]
pub struct HealthCheckCliProvider<H: daemon_slayer_core::health_check::HealthCheck + Clone + Send> {
    health_check: H,
    command: CommandConfig,
}

impl<H: daemon_slayer_core::health_check::HealthCheck + Clone + Send> HealthCheckCliProvider<H> {
    pub fn new(health_check: H) -> Self {
        Self {
            health_check,
            command: CommandConfig {
                action: None,
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: "health".to_string(),
                    help_text: "Check service health".to_string(),
                    hide: false,
                    children: vec![],
                },
            },
        }
    }
}

#[async_trait::async_trait]
impl<H: HealthCheck + Clone + Send + 'static> CommandProvider for HealthCheckCliProvider<H> {
    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        return match matched_command
            .as_ref()
            .map(|c| &c.matched_command.command_type)
        {
            Some(CommandType::Subcommand { name, .. }) if name == "health" => {
                Ok(match self.health_check.invoke().await {
                    Ok(()) => CommandOutput::handled("Healthy".to_owned()),
                    Err(e) => CommandOutput::handled(format!("Unhealthy: {e:?}")),
                })
            }
            _ => Ok(CommandOutput::unhandled()),
        };
    }

    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![&self.command]
    }
}
