use std::{collections::HashMap, error::Error, marker::PhantomData, ops::Deref};

use clap::{Arg, ArgAction, ArgMatches};
use tracing::info;

use crate::{
    platform::Manager,
    service_manager::{Service, ServiceHandler, ServiceManager},
};

pub struct Cli<'a, H>
where
    H: Service + ServiceHandler,
{
    _phantom: PhantomData<H>,
    manager: Manager,
    commands: CliCommands<'a>,
}

pub struct CliCommands<'a>(HashMap<&'static str, CliCommand<'a>>);

impl<'a> CliCommands<'a> {
    fn insert(&mut self, key: &'static str, value: CliCommand<'a>) {
        self.0.insert(key, value);
    }
}

impl<'a> Default for CliCommands<'a> {
    fn default() -> Self {
        let mut commands = HashMap::new();
        commands.insert(Commands::INSTALL, CliCommand::Subcommand(Commands::INSTALL));
        commands.insert(
            Commands::UNINSTALL,
            CliCommand::Subcommand(Commands::UNINSTALL),
        );
        commands.insert(Commands::START, CliCommand::Subcommand(Commands::START));
        commands.insert(Commands::STATUS, CliCommand::Subcommand(Commands::STATUS));
        commands.insert(Commands::STOP, CliCommand::Subcommand(Commands::STOP));
        commands.insert(Commands::RUN, CliCommand::Subcommand(Commands::RUN));
        #[cfg(feature = "direct")]
        commands.insert(Commands::DIRECT, CliCommand::Default);
        Self(commands)
    }
}

impl<'a> Deref for CliCommands<'a> {
    type Target = HashMap<&'static str, CliCommand<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum CliCommand<'a> {
    Subcommand(&'a str),
    Arg { short: char, long: &'a str },
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

impl<'a, H> Cli<'a, H>
where
    H: Service + ServiceHandler,
{
    pub fn new(manager: Manager) -> Self {
        let commands = CliCommands::default();
        Self {
            manager,
            commands,
            _phantom: PhantomData::default(),
        }
    }

    pub fn with_install_command(mut self, command: CliCommand<'a>) -> Self {
        self.commands.insert(Commands::INSTALL, command);
        self
    }

    pub fn with_uninstall_command(mut self, command: CliCommand<'a>) -> Self {
        self.commands.insert(Commands::UNINSTALL, command);
        self
    }

    pub fn with_start_command(mut self, command: CliCommand<'a>) -> Self {
        self.commands.insert(Commands::START, command);
        self
    }

    pub fn with_stop_command(mut self, command: CliCommand<'a>) -> Self {
        self.commands.insert(Commands::STOP, command);
        self
    }

    pub fn with_status_command(mut self, command: CliCommand<'a>) -> Self {
        self.commands.insert(Commands::STATUS, command);
        self
    }

    #[cfg(feature = "direct")]
    pub fn with_direct_command(mut self, command: CliCommand<'a>) -> Self {
        self.commands.insert(Commands::DIRECT, command);
        self
    }

    pub fn with_run_command(mut self, command: CliCommand<'a>) -> Self {
        self.commands.insert(Commands::RUN, command);
        self
    }

    fn matches(m: &ArgMatches, cmd: &CliCommand, cmd_name: &'static str) -> bool {
        match cmd {
            CliCommand::Arg { short: _, long: _ } => m.get_one::<bool>(cmd_name) == Some(&true),
            CliCommand::Subcommand(_) => m.subcommand().map(|r| r.0) == Some(cmd_name),
            CliCommand::Default => !m.args_present() && m.subcommand() == None,
        }
    }

    #[maybe_async::maybe_async]
    pub async fn handle_input(self) -> Result<(), Box<dyn Error>> {
        let mut cmd = clap::Command::new(self.manager.display_name());
        for (name, command) in self.commands.iter() {
            match command {
                CliCommand::Arg { short, long } => {
                    cmd = cmd.arg(
                        Arg::new(*name)
                            .short(*short)
                            .long(long)
                            .action(ArgAction::SetTrue),
                    )
                }
                CliCommand::Subcommand(subcommand) => {
                    cmd = cmd.subcommand(clap::command!(*subcommand))
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
