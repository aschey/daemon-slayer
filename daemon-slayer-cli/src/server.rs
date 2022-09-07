use crate::input_state::InputState;

maybe_async_cfg::content! {

    #![maybe_async_cfg::default(
        idents(
            ClientCli,
            ServerCli,
            CliHandler,
            ClientCliBuilder,
            ServerCliBuilder,
            Handler,
            Service
        )
    )]

use std::{error::Error, marker::PhantomData};
use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;
use crate::{
    action::Action, builder, cli_handler, command::Command, commands::Commands,
    service_commands::ServiceCommands, util,
};

#[maybe_async_cfg::maybe(sync(feature="blocking"), async(feature = "async-tokio"))]
pub struct ServerCli<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler,
{
    _phantom: PhantomData<H>,
    commands: Commands,
    display_name: String,
    description: String,
}

#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
impl<H> ServerCli<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler,
{
    pub fn builder(display_name: String, description: String) -> builder::ServerCliBuilder<H> {
        let commands = Commands::default();
        builder::ServerCliBuilder::new(display_name, description, commands)
    }

    pub(crate) fn from_builder(builder: builder::ServerCliBuilder<H>) -> Self {
        Self {
            commands: builder.commands,
            display_name: builder.display_name,
            description: builder.description,
            _phantom: PhantomData::default(),
        }
    }

    pub fn new(display_name: String, description: String) -> Self {
        let commands = Commands::default();

        Self {
            commands,
            display_name,
            description,
            _phantom: PhantomData::default(),
        }
    }



    pub(crate) fn commands(&self) -> &Commands {
        &self.commands
    }

    pub(crate) async fn handle_cmd(
        &self,
        matches: &ArgMatches,
    ) -> Result<bool, Box<dyn Error>> {
        for (name, cmd) in self.commands.iter() {
            if util::matches(matches, cmd, name) {
                info!("checking {name}");
                match *name {
                    ServiceCommands::RUN => {
                        info!("running...");
                        H::run_service_main().await;
                        return Ok(true);
                    }

                    #[cfg(feature = "direct")]
                    ServiceCommands::DIRECT => {
                        let handler = H::new();
                        handler.run_service_direct().await;
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
        Ok(false)
    }
}

#[maybe_async_cfg::maybe(
    idents(ServerCli, CliHandler),
    sync(cfg(feature = "blocking")),
    async(feature = "async-tokio",send="false", "async_trait::async_trait(?Send)"),
)]
impl<H> cli_handler::CliHandler for ServerCli<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler,
{
    async fn handle_input(self) -> Result<InputState, Box<dyn Error>> {
        let mut cmd = util::build_cmd(&self.display_name, &*self.description, self.commands.iter());
        let matches =cmd.get_matches();


        if self.handle_cmd(&matches).await? {
            Ok(InputState::Handled)
        } else {
            Ok(InputState::Unhandled(matches))
        }
    }

    fn action_type(&self) -> Action {
        let cmd = util::build_cmd(&self.display_name, &*self.description, self.commands.iter());
        let matches = &cmd.get_matches();
        for (name, cmd) in self.commands.iter() {
            if util::matches(matches, cmd, name) {
                if *name == ServiceCommands::RUN {
                    return Action::Server;
                }
                #[cfg(feature = "direct")]
                {
                    if *name == ServiceCommands::DIRECT {
                        return Action::Server;
                    }
                }
            }
        }
        Action::Unknown
    }
}

}
