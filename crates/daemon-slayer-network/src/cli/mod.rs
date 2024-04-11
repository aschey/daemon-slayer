use async_trait::async_trait;
use daemon_slayer_core::cli::clap::{self, Args, FromArgMatches, Subcommand};
use daemon_slayer_core::cli::{
    ActionType, CommandMatch, CommandOutput, CommandProvider, OwoColorize, Printer,
};
use daemon_slayer_core::server::background_service::BackgroundServiceManager;
use daemon_slayer_core::server::EventStore;
use daemon_slayer_core::{BoxedError, CancellationToken};
use futures::StreamExt;

use crate::mdns::{
    MdnsBroadcastName, MdnsBroadcastService, MdnsQueryName, MdnsQueryService, MdnsReceiverEvent,
};
use crate::{get_default_interface_from_route, get_default_route, ServiceProtocol};

#[derive(Subcommand, Clone, Debug)]
enum NetworkSubcommands {
    /// Show network routing info
    BroadcastInfo,
    /// Test mDNS send/receive
    MdnsTest,
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
                    let default_interface = get_default_interface_from_route(route)?;
                    let output = Printer::default()
                        .with_optional_line(
                            "Default IP",
                            default_interface.as_ref().map(|i| i.ip().to_string()),
                        )
                        .with_optional_line("Default Interface", default_interface.map(|i| i.name))
                        .with_line("Destination", route.destination.to_string())
                        .with_optional_line("Gateway", route.gateway.map(|g| g.to_string()));
                    CommandOutput::handled(output.print())
                } else {
                    CommandOutput::handled("Default route not found".to_string())
                }
            }
            NetworkSubcommands::MdnsTest => {
                let service_manager =
                    BackgroundServiceManager::new(CancellationToken::new(), Default::default());
                let mut context = service_manager.get_context();
                let mdns_broadcast_service = MdnsBroadcastService::new(
                    MdnsBroadcastName::new("test", "servicetest", ServiceProtocol::Tcp),
                    4321,
                    None,
                );
                let mut broadcast_events =
                    mdns_broadcast_service.get_event_store().subscribe_events();

                // let mut mdns_broadcast_events =
                //     mdns_broadcast_service.get_event_store().subscribe_events();
                context.add_service(mdns_broadcast_service);
                // tokio::time::sleep(Duration::from_millis(5000)).await;
                let mdns_query_service =
                    MdnsQueryService::new(MdnsQueryName::new("servicetest", ServiceProtocol::Tcp));
                let mut mdns_query_events = mdns_query_service.get_event_store().subscribe_events();
                context.add_service(mdns_query_service);

                let mut service_resolved = false;
                while let Some(Ok(query_event)) = mdns_query_events.next().await {
                    if let MdnsReceiverEvent::ServiceResolved(_) = query_event {
                        service_resolved = true;
                        service_manager.stop();
                    }
                }

                service_manager.cancel().await.unwrap();
                while let Some(Ok(_)) = broadcast_events.next().await {}
                if service_resolved {
                    CommandOutput::handled("success - service resolved\n".green().to_string())
                } else {
                    CommandOutput::handled("error - service not resolved\n".red().to_string())
                }
            }
        });
    }
}
