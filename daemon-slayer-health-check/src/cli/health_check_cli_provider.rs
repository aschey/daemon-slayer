use daemon_slayer_core::cli::{ActionType, ArgMatchesExt, CommandType, InputState};

pub struct HealthCheckCliProvider<H: daemon_slayer_core::health_check::HealthCheck + Send> {
    health_check: H,
    command: CommandType,
}

impl<H: daemon_slayer_core::health_check::HealthCheck + Send> HealthCheckCliProvider<H> {
    pub fn new(health_check: H) -> Self {
        Self {
            health_check,
            command: CommandType::Subcommand {
                name: "health".to_string(),
                help_text: "Check service health".to_string(),
                hide: false,
                children: None,
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
        matches: &daemon_slayer_core::cli::clap::ArgMatches,
    ) -> InputState {
        if matches.matches(&self.command) {
            match self.health_check.invoke().await {
                Ok(()) => println!("Healthy"),
                Err(e) => {
                    println!("Unhealthy: {e:?}");
                }
            }
            return InputState::Handled;
        }
        InputState::Unhandled
    }

    fn get_action_type(&self) -> daemon_slayer_core::cli::ActionType {
        ActionType::Client
    }

    fn get_commands(&self) -> Vec<&daemon_slayer_core::cli::CommandType> {
        vec![&self.command]
    }
}
