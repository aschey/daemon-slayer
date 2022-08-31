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
            CliCommand::Subcommand(Commands::INSTALL.to_owned()),
        );
        commands.insert(
            Commands::UNINSTALL,
            CliCommand::Subcommand(Commands::UNINSTALL.to_owned()),
        );
        commands.insert(
            Commands::START,
            CliCommand::Subcommand(Commands::START.to_owned()),
        );
        commands.insert(
            Commands::STATUS,
            CliCommand::Subcommand(Commands::STATUS.to_owned()),
        );
        commands.insert(
            Commands::STOP,
            CliCommand::Subcommand(Commands::STOP.to_owned()),
        );
        commands.insert(
            Commands::RUN,
            CliCommand::Subcommand(Commands::RUN.to_owned()),
        );
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
    Subcommand(String),
    Arg { short: char, long: String },
    Default,
}

struct Commands;

impl Commands {
    const INSTALL: &'static str = "install";
    const UNINSTALL: &'static str = "uninstall";
    const RUN: &'static str = "run";
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
        let commands = CliCommands::default();
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
                    cmd = cmd.subcommand(clap::command!(subcommand))
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
