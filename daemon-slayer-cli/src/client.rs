use std::{error::Error, marker::PhantomData};

use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;

use daemon_slayer_client::{Manager, ServiceManager};

use crate::{
    builder::Builder, command::Command, commands::Commands, service_commands::ServiceCommands, util,
};

pub struct Cli {
    manager: ServiceManager,
    commands: Commands,
}

impl Cli {
    #[cfg(all(not(feature = "server"), feature = "client"))]
    pub fn builder(manager: ServiceManager) -> Builder {
        let commands = Commands::default();
        Builder::from_manager(manager, commands)
    }

    #[cfg(all(not(feature = "server"), feature = "client"))]
    pub(crate) fn from_builder(builder: Builder) -> Self {
        Self {
            manager: builder.manager.unwrap(),
            commands: builder.commands,
        }
    }

    pub fn new(manager: ServiceManager) -> Self {
        let mut commands = Commands::default();
        let service_args = manager.args();
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

    pub(crate) fn commands(&self) -> &Commands {
        &self.commands
    }

    #[maybe_async::maybe_async]
    pub(crate) async fn handle_cmd(&self, matches: &ArgMatches) -> Result<bool, Box<dyn Error>> {
        for (name, cmd) in self.commands.iter() {
            if Self::matches(&matches, cmd, name) {
                info!("checking {name}");
                match *name {
                    ServiceCommands::INSTALL => {
                        info!("installing...");
                        self.manager.install()?;
                        return Ok(true);
                    }
                    ServiceCommands::UNINSTALL => {
                        info!("uninstalling...");
                        self.manager.uninstall()?;
                        return Ok(true);
                    }
                    ServiceCommands::STATUS => {
                        println!("{:?}", self.manager.query_status()?);
                        return Ok(true);
                    }
                    ServiceCommands::START => {
                        info!("starting...");
                        self.manager.start()?;
                        return Ok(true);
                    }
                    ServiceCommands::STOP => {
                        info!("stopping..");
                        self.manager.stop()?;
                        return Ok(true);
                    }

                    #[cfg(feature = "console")]
                    ServiceCommands::CONSOLE => {
                        //crate::console::run()?;
                    }

                    _ => {}
                }
            }
        }

        Ok(false)
    }

    #[maybe_async::maybe_async]
    pub async fn handle_input(self) -> Result<bool, Box<dyn Error>> {
        let mut cmd = util::build_cmd(
            &self.manager.display_name(),
            &*self.manager.description(),
            self.commands.iter(),
        );
        self.handle_cmd(&cmd.get_matches()).await
    }
}
