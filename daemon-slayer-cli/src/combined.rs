use std::{error::Error, marker::PhantomData};

use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;

use daemon_slayer_client::{Manager, ServiceManager};
use daemon_slayer_server::{Handler, Service};

use crate::{
    builder::Builder, client, command::Command, commands::Commands, server,
    service_commands::ServiceCommands, util,
};

pub struct Cli<H>
where
    H: Service + Handler,
{
    client_cli: client::Cli,
    server_cli: server::Cli<H>,
    display_name: String,
    description: String,
    commands: Commands,
}

impl<H> Cli<H>
where
    H: Service + Handler,
{
    pub fn builder(manager: ServiceManager) -> Builder<H> {
        let commands = Commands::default();
        Builder::from_manager(manager, commands)
    }

    pub(crate) fn from_builder(builder: Builder<H>) -> Self {
        let display_name = builder.display_name.clone();
        let description = builder.description.clone();
        Self {
            client_cli: client::Cli::new(builder.manager.unwrap()),
            server_cli: server::Cli::new(builder.display_name, builder.description),
            commands: builder.commands,
            display_name,
            description,
        }
    }

    pub fn new(manager: ServiceManager) -> Self {
        let display_name = manager.display_name().to_string();
        let description = manager.description().to_string();
        let client_cli = client::Cli::new(manager);
        let commands = Commands::default();
        let server_cli = server::Cli::<H>::new(display_name.clone(), description.clone());

        Self {
            client_cli,
            server_cli,
            display_name,
            description,
            commands,
        }
    }

    #[maybe_async::maybe_async]
    pub async fn handle_input(self) -> Result<bool, Box<dyn Error>> {
        let cmd = util::build_cmd(&self.display_name, &self.description, self.commands.iter());
        let matches = cmd.get_matches();
        match self.server_cli.handle_cmd(&matches).await {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(e) => return Err(e),
        };

        self.client_cli.handle_cmd(&matches).await
    }
}
