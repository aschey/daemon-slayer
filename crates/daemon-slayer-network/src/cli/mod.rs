use daemon_slayer_core::cli::clap::{self, Args, FromArgMatches, Subcommand};
use daemon_slayer_core::cli::{ActionType, CommandMatch, CommandOutput, CommandProvider, Printer};
use daemon_slayer_core::{async_trait, BoxedError};

use crate::{get_default_ip_from_route, get_default_route};

#[derive(Subcommand, Clone, Debug)]
enum NetworkSubcommands {
    /// Show network routing info
    BroadcastInfo,
}

#[derive(Args, Clone, Debug)]
struct NetworkArgs {
    #[command(subcommand)]
    commands: NetworkSubcommands,
}

#[derive(Subcommand)]
enum CliCommands {
    /// View network information
    Network(NetworkArgs),
}

#[derive(Clone, Debug, Default)]
pub struct NetworkCliProvider {
    matched_args: Option<NetworkArgs>,
}

impl NetworkCliProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CommandProvider for NetworkCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliCommands::augment_subcommands(command)
    }

    fn matches(&mut self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let command = CliCommands::from_arg_matches(matches).ok()?;
        let CliCommands::Network(args) = command;
        self.matched_args = Some(args);
        Some(CommandMatch {
            action_type: ActionType::Other,
            action: None,
        })
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        let Some(args) = self.matched_args else {
            return Ok(CommandOutput::unhandled());
        };

        return Ok(match args.commands {
            NetworkSubcommands::BroadcastInfo => {
                let route = get_default_route().await?;
                if let Some(route) = &route {
                    let default_ip = get_default_ip_from_route(route)?.map(|ip| ip.to_string());
                    let output = Printer::default()
                        .with_optional_line("Default IP", default_ip)
                        .with_line("Destination", route.destination.to_string())
                        .with_optional_line("Gateway", route.gateway.map(|g| g.to_string()));
                    CommandOutput::handled(output.print())
                } else {
                    CommandOutput::handled("Default route not found".to_string())
                }
            }
        });
    }
}
