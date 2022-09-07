use crate::input_state::InputState;

maybe_async_cfg::content! {

    #![maybe_async_cfg::default(
        idents(
            ClientCli,
            ServerCli,
            CliHandler,
            CliBuilder,
            Handler,
            Service
        )
    )]

use std::{error::Error, marker::PhantomData};

use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;

use daemon_slayer_client::{Manager, ServiceManager};


use crate::{
    action::Action, builder, client, cli_handler, command::Command, commands::Commands,
    server, service_commands::ServiceCommands, util,
};

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
pub struct Cli<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler,
{
    client_cli: client::ClientCli,
    server_cli: server::ServerCli<H>,
    display_name: String,
    description: String,
    commands: Commands,
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
impl<H> Cli<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler ,
{
    pub fn builder(manager: ServiceManager) -> builder::CliBuilder<H> {
        let commands = Commands::default();
        builder::CliBuilder::from_manager(manager, commands)
    }

    pub fn new(manager: ServiceManager) -> Self {
        Self::builder(manager).build()
    }

    pub(crate) fn from_builder(builder: builder::CliBuilder<H>) -> Self {
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

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio", "async_trait::async_trait(?Send)"),
)]
impl<H> cli_handler::CliHandler for Cli<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler,
{
    async fn handle_input(self) -> Result<InputState, Box<dyn Error>> {
        let cmd = util::build_cmd(&self.display_name, &self.description, self.commands.iter());
        let matches = cmd.get_matches();
        if self.server_cli.handle_cmd(&matches).await? {
          return Ok(InputState::Handled);
        };

        if self.client_cli.handle_cmd(&matches).await? {
            Ok(InputState::Handled)
        } else {
            Ok(InputState::Unhandled(matches))
        }
    }

    fn action_type(&self) -> Action {
        if self.server_cli.action_type() == Action::Server {
            return Action::Server;
        }
        self.client_cli.action_type()
    }
}
}
