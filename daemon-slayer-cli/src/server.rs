use std::{error::Error, marker::PhantomData};

use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;

use daemon_slayer_server::{Handler, Service};

use crate::{
    builder::Builder, command::Command, commands::Commands, service_commands::ServiceCommands, util,
};

pub struct Cli<H>
where
    H: Service + Handler,
{
    _phantom: PhantomData<H>,
    commands: Commands,
    display_name: String,
    description: String,
}

impl<H> Cli<H>
where
    H: Service + Handler,
{
    pub fn builder(display_name: String, description: String) -> Builder<H> {
        let commands = Commands::default();
        Builder::new(display_name, description, commands)
    }

    pub(crate) fn from_builder(builder: Builder<H>) -> Self {
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

    fn matches(m: &ArgMatches, cmd: &Command, cmd_name: &'static str) -> bool {
        match cmd {
            Command::Arg {
                short: _,
                long: _,
                help_text: _,
            } => m.get_one::<bool>(cmd_name) == Some(&true),
            Command::Subcommand {
                name: _,
                help_text: _,
            } => m.subcommand().map(|r| r.0) == Some(cmd_name),
            Command::Default => !m.args_present() && m.subcommand() == None,
        }
    }

    pub(crate) fn commands(&self) -> &Commands {
        &self.commands
    }

    #[maybe_async::maybe_async]
    pub(crate) async fn handle_cmd<'a>(
        &self,
        matches: &ArgMatches,
    ) -> Result<bool, Box<dyn Error>> {
        for (name, cmd) in self.commands.iter() {
            if Self::matches(matches, cmd, name) {
                info!("checking {name}");
                match *name {
                    ServiceCommands::RUN => {
                        info!("running...");
                        H::run_service_main().await;
                        return Ok(true);
                    }
                    #[cfg(feature = "console")]
                    ServiceCommands::CONSOLE => {
                        //crate::console::run()?;
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

    #[maybe_async::maybe_async]
    pub async fn handle_input(self) -> Result<bool, Box<dyn Error>> {
        let mut cmd = util::build_cmd(&self.display_name, &*self.description, self.commands.iter());
        self.handle_cmd(&cmd.get_matches()).await
    }
}
