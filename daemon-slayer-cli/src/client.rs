use crate::input_state::InputState;

maybe_async_cfg::content! {

    #![maybe_async_cfg::default(
        idents(
            ClientCli,
            ServerCli,
            CliHandler,
            ClientCliBuilder
        )
    )]

use std::{error::Error};
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
            if util::matches($matches, cmd, name) {
                match *name {
                    ServiceCommands::INSTALL => {
                        $self.manager.install()?;
                        return Ok(true);
                    }
                    ServiceCommands::UNINSTALL => {
                        $self.manager.uninstall()?;
                        return Ok(true);
                    }
                    ServiceCommands::INFO => {
                        println!("{:?}", $self.manager.info()?);
                        return Ok(true);
                    }
                    ServiceCommands::PID => {
                        println!("{}", $self.manager.info()?.pid.map(|pid| pid.to_string()).unwrap_or("".to_owned()));
                        return Ok(true);
                    }
                    ServiceCommands::START => {
                        $self.manager.start()?;
                        return Ok(true);
                    }
                    ServiceCommands::STOP => {
                        $self.manager.stop()?;
                        return Ok(true);
                    }
                    ServiceCommands::RESTART => {
                        $self.manager.restart()?;
                        return Ok(true);
                    }
                    ServiceCommands::ENABLE => {
                        $self.manager.set_autostart_enabled(true)?;
                        return Ok(true);
                    }
                    ServiceCommands::DISABLE => {
                        $self.manager.set_autostart_enabled(false)?;
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
    commands: Commands
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

        Self { manager, commands, }
    }



    #[maybe_async_cfg::only_if(async)]
    pub(crate) async fn handle_cmd(mut self, matches: &ArgMatches) -> Result<bool, Box<dyn Error>> {
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
    pub(crate) async fn handle_cmd(mut self, matches: &ArgMatches) -> Result<bool, Box<dyn Error>> {
        get_handlers!(self, matches,);

        Ok(false)
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio", "async_trait::async_trait(?Send)")
)]
impl cli_handler::CliHandler for ClientCli {
    async fn handle_input(self) -> Result<InputState, Box<dyn Error>> {
        let cmd = util::build_cmd(
            self.manager.display_name(),
            self.manager.description(),
            self.commands.iter(),
        );
        let matches = cmd.get_matches();

        if self.handle_cmd(&matches).await? {
            Ok(InputState::Handled)
        } else {
            Ok(InputState::Unhandled(matches))
        }
    }

    fn action_type(&self) -> Action {
        let cmd = util::build_cmd(
            self.manager.display_name(),
            self.manager.description(),
            self.commands.iter(),
        );
        let matches = &cmd.get_matches();
        for (name, cmd) in self.commands.iter() {
            if util::matches(matches, cmd, name) {
                match *name {
                    ServiceCommands::INSTALL
                    | ServiceCommands::UNINSTALL
                    | ServiceCommands::INFO
                    | ServiceCommands::START
                    | ServiceCommands::STOP
                    | ServiceCommands::RESTART
                    | ServiceCommands::PID
                    | ServiceCommands::ENABLE
                    | ServiceCommands::DISABLE => {
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
