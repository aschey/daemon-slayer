use daemon_slayer_core::cli::{
    clap, ActionType, ArgMatchesExt, CommandConfig, CommandType, InputState,
};

pub struct HealthCheckCliProvider<H: daemon_slayer_core::health_check::HealthCheck + Send> {
    health_check: H,
    command: CommandConfig,
}

impl<H: daemon_slayer_core::health_check::HealthCheck + Send> HealthCheckCliProvider<H> {
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
                    children: None,
                },
            },
        }
    }
}

#[async_trait::async_trait]
impl<H: daemon_slayer_core::health_check::HealthCheck + Send>
    daemon_slayer_core::cli::CommandProvider for HealthCheckCliProvider<H>
{
    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandConfig>,
    ) -> InputState {
        match matched_command.as_ref().map(|c| &c.command_type) {
            Some(CommandType::Subcommand {
                name,
                help_text: _,
                hide: _,
                children: _,
            }) if name == "health" => {
                match self.health_check.invoke().await {
                    Ok(()) => println!("Healthy"),
                    Err(e) => {
                        println!("Unhealthy: {e:?}");
                    }
                }
                InputState::Handled
            }
            _ => InputState::Unhandled,
        }
    }

    fn get_action_type(&self) -> daemon_slayer_core::cli::ActionType {
        ActionType::Client
    }

    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![&self.command]
    }
}
