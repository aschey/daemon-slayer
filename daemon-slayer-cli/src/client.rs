maybe_async_cfg::content! {

    #![maybe_async_cfg::default(
        idents(
            ClientCli,
            ServerCli,
            CliHandler,
            ClientCliBuilder
        )
    )]

use std::{error::Error, marker::PhantomData};

use clap::{Arg, ArgAction, ArgMatches};
#[cfg(feature = "console")]
use daemon_slayer_console::Console;
use tracing::info;

use daemon_slayer_client::{Manager, ServiceManager};

use crate::{
    action::Action, builder, command::Command, commands::Commands,
    service_commands::ServiceCommands, util, cli_handler
};

macro_rules! get_handlers {
    ($self: ident, $matches: ident, $($extra:tt)*) => {
        for (name, cmd) in $self.commands.iter() {
            if Self::matches($matches, cmd, name) {
                info!("checking {name}");
                match *name {
                    ServiceCommands::INSTALL => {
                        info!("installing...");
                        $self.manager.install()?;
                        return Ok(true);
                    }
                    ServiceCommands::UNINSTALL => {
                        info!("uninstalling...");
                        $self.manager.uninstall()?;
                        return Ok(true);
                    }
                    ServiceCommands::STATUS => {
                        println!("{:?}", $self.manager.query_status()?);
                        return Ok(true);
                    }
                    ServiceCommands::START => {
                        info!("starting...");
                        $self.manager.start()?;
                        return Ok(true);
                    }
                    ServiceCommands::STOP => {
                        info!("stopping..");
                        $self.manager.stop()?;
                        return Ok(true);
                    }

                    $($extra)*

                    _ => {}
                }
            }
        }
    }
}

#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
pub struct ClientCli {
    manager: ServiceManager,
    commands: Commands,
}

#[maybe_async_cfg::maybe(sync(feature = "blocking"), async(feature = "async-tokio"))]
impl ClientCli {
    pub fn builder(manager: ServiceManager) -> builder::ClientCliBuilder {
        let commands = Commands::default();
        builder::ClientCliBuilder::from_manager(manager, commands)
    }

    pub(crate) fn from_builder(builder: builder::ClientCliBuilder) -> Self {
        Self {
            manager: builder.manager,
            commands: builder.commands,
        }
    }

    pub fn new(manager: ServiceManager) -> Self {
        let commands = Commands::default();

        Self { manager, commands }
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

    #[maybe_async_cfg::only_if(async)]
    pub(crate) async fn handle_cmd(self, matches: &ArgMatches) -> Result<bool, Box<dyn Error>> {
        get_handlers!(self, matches,
            #[cfg(feature="console")]
            ServiceCommands::CONSOLE => {
            let mut console = Console::new(self.manager);
            console.run().await?;
            return Ok(true);
        });

        Ok(false)
    }

    #[maybe_async_cfg::only_if(sync)]
    pub(crate) async fn handle_cmd(self, matches: &ArgMatches) -> Result<bool, Box<dyn Error>> {
        get_handlers!(self, matches,);

        Ok(false)
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio", "async_trait::async_trait(?Send)")
)]
impl cli_handler::CliHandler for ClientCli {
    async fn handle_input(self) -> Result<bool, Box<dyn Error>> {
        let cmd = util::build_cmd(
            self.manager.display_name(),
            self.manager.description(),
            self.commands.iter(),
        );
        let matches = cmd.get_matches();
        self.handle_cmd(&matches).await
    }

    fn action_type(&self) -> Action {
        let cmd = util::build_cmd(
            self.manager.display_name(),
            self.manager.description(),
            self.commands.iter(),
        );
        let matches = &cmd.get_matches();
        for (name, cmd) in self.commands.iter() {
            if Self::matches(matches, cmd, name) {
                match *name {
                    ServiceCommands::INSTALL
                    | ServiceCommands::UNINSTALL
                    | ServiceCommands::STATUS
                    | ServiceCommands::START
                    | ServiceCommands::STOP => {
                        return Action::Client;
                    }
                    #[cfg(feature = "console")]
                    ServiceCommands::CONSOLE => return Action::Client,
                    _ => return Action::Unknown,
                }
            }
        }
        Action::Unknown
    }
}
}