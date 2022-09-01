use std::{error::Error, marker::PhantomData};

use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;

use daemon_slayer_client::{Manager, ServiceManager};
use daemon_slayer_server::{Handler, Service};

use crate::{command::Command, commands::Commands, service_commands::ServiceCommands};

pub struct Cli<H>
where
    H: Service + Handler,
{
    _phantom: PhantomData<H>,
    manager: ServiceManager,
    commands: Commands,
}

impl<H> Cli<H>
where
    H: Service + Handler,
{
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

        Self {
            manager,
            commands,
            _phantom: PhantomData::default(),
        }
    }

    pub fn with_install_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::INSTALL, command);
        self
    }

    pub fn with_uninstall_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::UNINSTALL, command);
        self
    }

    pub fn with_start_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::START, command);
        self
    }

    pub fn with_stop_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::STOP, command);
        self
    }

    pub fn with_status_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::STATUS, command);
        self
    }

    #[cfg(feature = "direct")]
    pub fn with_direct_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::DIRECT, command);
        self
    }

    #[cfg(feature = "console")]
    pub fn with_console_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::CONSOLE, command);
        self
    }

    pub fn with_run_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::RUN, command);
        self
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

    #[maybe_async::maybe_async]
    pub async fn handle_input(self) -> Result<(), Box<dyn Error>> {
        let mut cmd =
            clap::Command::new(self.manager.display_name()).about(self.manager.description());
        for (name, command) in self.commands.iter() {
            let hide = (*name) == ServiceCommands::RUN;
            match command {
                Command::Arg {
                    short,
                    long,
                    help_text,
                } => {
                    let mut arg = Arg::new(*name);
                    if let Some(short) = short {
                        arg = arg.short(*short);
                    }
                    if let Some(long) = long {
                        arg = arg.long(long);
                    }

                    cmd = cmd.arg(
                        arg.action(ArgAction::SetTrue)
                            .help(help_text.as_ref().map(&String::as_ref))
                            .hide(hide),
                    )
                }
                Command::Subcommand { name, help_text } => {
                    cmd = cmd.subcommand(clap::command!(name).about(&**help_text).hide(hide))
                }
                Command::Default => {}
            }
        }
        let matches = cmd.get_matches();
        for (name, cmd) in self.commands.iter() {
            if Self::matches(&matches, cmd, name) {
                info!("checking {name}");
                match *name {
                    ServiceCommands::INSTALL => {
                        info!("installing...");
                        self.manager.install()?;
                    }
                    ServiceCommands::UNINSTALL => {
                        info!("uninstalling...");
                        self.manager.uninstall()?;
                    }
                    ServiceCommands::STATUS => {
                        println!("{:?}", self.manager.query_status()?);
                    }
                    ServiceCommands::START => {
                        info!("starting...");
                        self.manager.start()?;
                    }
                    ServiceCommands::STOP => {
                        info!("stopping..");
                        self.manager.stop()?;
                    }
                    ServiceCommands::RUN => {
                        info!("running...");
                        H::run_service_main().await;
                    }
                    #[cfg(feature = "console")]
                    ServiceCommands::CONSOLE => {
                        //crate::console::run()?;
                    }
                    #[cfg(feature = "direct")]
                    ServiceCommands::DIRECT => {
                        let handler = H::new();
                        handler.run_service_direct().await;
                    }
                    _ => {
                        info!("unknown command");
                    }
                }
                return Ok(());
            }
        }

        Ok(())
    }
}
