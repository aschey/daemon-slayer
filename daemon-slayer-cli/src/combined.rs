use std::{error::Error, marker::PhantomData};

use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;

use daemon_slayer_client::{Manager, ServiceManager};
use daemon_slayer_server::{Handler, Service};

use crate::{
    action::Action, builder::CliBuilder, cli_handler::CliHandler, client, command::Command,
    commands::Commands, server, service_commands::ServiceCommands, util,
};

pub struct Cli<H>
where
    H: Service + Handler,
{
    client_cli: client::ClientCli,
    server_cli: server::ServerCli<H>,
    display_name: String,
    description: String,
    commands: Commands,
}

impl<H> Cli<H>
where
    H: Service + Handler,
{
    pub fn builder(manager: ServiceManager) -> CliBuilder<H> {
        let commands = Commands::default();
        CliBuilder::from_manager(manager, commands)
    }

    pub fn new(manager: ServiceManager) -> Self {
        Self::builder(manager).build()
    }

    pub(crate) fn from_builder(builder: CliBuilder<H>) -> Self {
        let manager = builder.manager.unwrap();
        let service_args = manager.args();
        let mut commands = builder.commands;
        if service_args.is_empty() {
            commands.insert(ServiceCommands::RUN, Command::Default);
            #[cfg(feature = "direct")]
            commands.insert(
                ServiceCommands::DIRECT,
                Command::Subcommand {
                    name: ServiceCommands::DIRECT.to_owned(),
                    help_text: "Run the service directly".to_owned(),
                },
            );
        } else {
            // Already checked that args is not empty so this shouldn't fail
            let first = service_args.first().unwrap();
            if first.starts_with("--") {
                commands.insert(
                    ServiceCommands::RUN,
                    Command::Arg {
                        short: None,
                        long: Some(first.to_owned()),
                        help_text: None,
                    },
                );
            } else if first.starts_with('-') {
                commands.insert(
                    ServiceCommands::RUN,
                    Command::Arg {
                        short: Some(first.replacen('-', "", 1).chars().next().unwrap()),
                        long: None,
                        help_text: None,
                    },
                );
            }
        }
        let display_name = builder.display_name.clone();
        let description = builder.description.clone();
        Self {
            client_cli: client::ClientCli::new(manager),
            server_cli: server::ServerCli::new(builder.display_name, builder.description),
            commands,
            display_name,
            description,
        }
    }
}

#[maybe_async::maybe_async(?Send)]
impl<H> CliHandler for Cli<H>
where
    H: Service + Handler,
{
    async fn handle_input(self) -> Result<bool, Box<dyn Error>> {
        let cmd = util::build_cmd(&self.display_name, &self.description, self.commands.iter());
        let matches = cmd.get_matches();
        match self.server_cli.handle_cmd(&matches).await {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(e) => return Err(e),
        };

        self.client_cli.handle_cmd(&matches).await
    }

    fn action_type(&self) -> Action {
        if self.server_cli.action_type() == Action::Server {
            return Action::Server;
        }
        self.client_cli.action_type()
    }
}
