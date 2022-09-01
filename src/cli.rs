use std::{collections::HashMap, error::Error, marker::PhantomData, ops::Deref};

use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;

use crate::{
    platform::Manager,
    service_manager::{Service, ServiceHandler, ServiceManager},
};

pub struct Cli<H>
where
    H: Service + ServiceHandler,
{
    _phantom: PhantomData<H>,
    manager: Manager,
    commands: CliCommands,
}

pub struct CliCommands(HashMap<&'static str, CliCommand>);

impl CliCommands {
    fn insert(&mut self, key: &'static str, value: CliCommand) {
        self.0.insert(key, value);
    }
}

impl Default for CliCommands {
    fn default() -> Self {
        let mut commands = HashMap::new();
        commands.insert(
            Commands::INSTALL,
            CliCommand::Subcommand {
                name: Commands::INSTALL.to_owned(),
                help_text: "Install the service using the system's service manager".to_owned(),
            },
        );
        commands.insert(
            Commands::UNINSTALL,
            CliCommand::Subcommand {
                name: Commands::UNINSTALL.to_owned(),
                help_text: "Uninstall the service from the system's service manager".to_owned(),
            },
        );
        commands.insert(
            Commands::START,
            CliCommand::Subcommand {
                name: Commands::START.to_owned(),
                help_text: "Start the service".to_owned(),
            },
        );
        commands.insert(
            Commands::STATUS,
            CliCommand::Subcommand {
                name: Commands::STATUS.to_owned(),
                help_text: "Get the service's current status".to_owned(),
            },
        );
        commands.insert(
            Commands::STOP,
            CliCommand::Subcommand {
                name: Commands::STOP.to_owned(),
                help_text: "Stop the service".to_owned(),
            },
        );
        commands.insert(
            Commands::RUN,
            CliCommand::Subcommand {
                name: Commands::RUN.to_owned(),
                help_text: "".to_owned(),
            },
        );
        #[cfg(feature = "direct")]
        commands.insert(Commands::DIRECT, CliCommand::Default);
        Self(commands)
    }
}

impl Deref for CliCommands {
    type Target = HashMap<&'static str, CliCommand>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum CliCommand {
    Subcommand {
        name: String,
        help_text: String,
    },
    Arg {
        short: Option<char>,
        long: Option<String>,
        help_text: Option<String>,
    },
    Default,
}

struct Commands;

impl Commands {
    const INSTALL: &'static str = "install";
    const UNINSTALL: &'static str = "uninstall";
    const RUN: &'static str = "run";
    #[cfg(feature = "direct")]
    const DIRECT: &'static str = "direct";
    const STATUS: &'static str = "status";
    const START: &'static str = "start";
    const STOP: &'static str = "stop";
}

impl<H> Cli<H>
where
    H: Service + ServiceHandler,
{
    pub fn new(manager: Manager) -> Self {
        let mut commands = CliCommands::default();
        let service_args = manager.args();
        if service_args.is_empty() {
            commands.insert(Commands::RUN, CliCommand::Default);
            #[cfg(feature = "direct")]
            commands.insert(
                Commands::DIRECT,
                CliCommand::Subcommand {
                    name: Commands::DIRECT.to_owned(),
                    help_text: "Run the service directly".to_owned(),
                },
            )
        } else {
            // Already checked that args is not empty so this shouldn't fail
            let first = service_args.first().unwrap();
            if first.starts_with("--") {
                commands.insert(
                    Commands::RUN,
                    CliCommand::Arg {
                        short: None,
                        long: Some(first.to_owned()),
                        help_text: None,
                    },
                )
            } else if first.starts_with('-') {
                commands.insert(
                    Commands::RUN,
                    CliCommand::Arg {
                        short: Some(first.replacen('-', "", 1).chars().next().unwrap()),
                        long: None,
                        help_text: None,
                    },
                )
            }
        }

        Self {
            manager,
            commands,
            _phantom: PhantomData::default(),
        }
    }

    pub fn with_install_command(mut self, command: CliCommand) -> Self {
        self.commands.insert(Commands::INSTALL, command);
        self
    }

    pub fn with_uninstall_command(mut self, command: CliCommand) -> Self {
        self.commands.insert(Commands::UNINSTALL, command);
        self
    }

    pub fn with_start_command(mut self, command: CliCommand) -> Self {
        self.commands.insert(Commands::START, command);
        self
    }

    pub fn with_stop_command(mut self, command: CliCommand) -> Self {
        self.commands.insert(Commands::STOP, command);
        self
    }

    pub fn with_status_command(mut self, command: CliCommand) -> Self {
        self.commands.insert(Commands::STATUS, command);
        self
    }

    #[cfg(feature = "direct")]
    pub fn with_direct_command(mut self, command: CliCommand) -> Self {
        self.commands.insert(Commands::DIRECT, command);
        self
    }

    pub fn with_run_command(mut self, command: CliCommand) -> Self {
        self.commands.insert(Commands::RUN, command);
        self
    }

    fn matches(m: &ArgMatches, cmd: &CliCommand, cmd_name: &'static str) -> bool {
        match cmd {
            CliCommand::Arg {
                short: _,
                long: _,
                help_text: _,
            } => m.get_one::<bool>(cmd_name) == Some(&true),
            CliCommand::Subcommand {
                name: _,
                help_text: _,
            } => m.subcommand().map(|r| r.0) == Some(cmd_name),
            CliCommand::Default => !m.args_present() && m.subcommand() == None,
        }
    }

    #[maybe_async::maybe_async]
    pub async fn handle_input(self) -> Result<(), Box<dyn Error>> {
        let mut cmd =
            clap::Command::new(self.manager.display_name()).about(self.manager.description());
        for (name, command) in self.commands.iter() {
            let hide = (*name) == Commands::RUN;
            match command {
                CliCommand::Arg {
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
                CliCommand::Subcommand { name, help_text } => {
                    cmd = cmd.subcommand(clap::command!(name).about(&**help_text).hide(hide))
                }
                CliCommand::Default => {}
            }
        }
        let matches = cmd.get_matches();
        for (name, cmd) in self.commands.iter() {
            if Self::matches(&matches, cmd, name) {
                info!("checking {name}");
                match *name {
                    Commands::INSTALL => {
                        info!("installing...");
                        self.manager.install()?;
                    }
                    Commands::UNINSTALL => {
                        info!("uninstalling...");
                        self.manager.uninstall()?;
                    }
                    Commands::STATUS => {
                        println!("{:?}", self.manager.query_status()?);
                    }
                    Commands::START => {
                        info!("starting...");
                        self.manager.start()?;
                    }
                    Commands::STOP => {
                        info!("stopping..");
                        self.manager.stop()?;
                    }
                    Commands::RUN => {
                        info!("running...");
                        H::run_service_main().await;
                    }
                    #[cfg(feature = "direct")]
                    Commands::DIRECT => {
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
