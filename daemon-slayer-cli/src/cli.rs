use std::error::Error;

use crate::{commands::Commands, service_commands::ServiceCommands, Action, Command, InputState};
use clap::{Arg, ArgAction, ArgMatches};
#[cfg(feature = "client")]
use daemon_slayer_client::{Manager, ServiceManager};
use tracing::info;

macro_rules! get_handlers {
    ($self: ident, $matches: ident, $($extra:tt)*) => {
        for (name, cmd) in $self.builder.commands.iter() {
            if $self.matches($matches, cmd, name) {
                #[cfg(feature = "client")]
                if let Some(manager) = &mut $self.builder.manager {
                    match *name {
                        ServiceCommands::INSTALL => {
                            manager.install()?;
                            return Ok(true);
                        }
                        ServiceCommands::UNINSTALL => {
                            manager.uninstall()?;
                            return Ok(true);
                        }
                        ServiceCommands::INFO => {
                            println!("{:?}", manager.info()?);
                            return Ok(true);
                        }
                        ServiceCommands::PID => {
                            println!(
                                "{}",
                                manager
                                    .info()?
                                    .pid
                                    .map(|pid| pid.to_string())
                                    .unwrap_or_else(|| "".to_owned())
                            );
                            return Ok(true);
                        }
                        ServiceCommands::START => {
                            manager.start()?;
                            return Ok(true);
                        }
                        ServiceCommands::STOP => {
                            manager.stop()?;
                            return Ok(true);
                        }
                        ServiceCommands::RESTART => {
                            manager.restart()?;
                            return Ok(true);
                        }
                        ServiceCommands::ENABLE => {
                            manager.set_autostart_enabled(true)?;
                            return Ok(true);
                        }
                        ServiceCommands::DISABLE => {
                            manager.set_autostart_enabled(false)?;
                            return Ok(true);
                        }

                        $($extra)*

                        _ => {}
                    }
                }
            }
        }
    }
}

#[maybe_async_cfg::maybe(
    idents(Service, Builder),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
pub struct Cli {
    builder: super::Builder,
}

#[maybe_async_cfg::maybe(
    idents(Service, Builder),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
impl Cli {
    #[cfg(all(feature = "client", feature = "server"))]
    pub fn for_all(
        manager: ServiceManager,
        service: impl daemon_slayer_server::Service + 'static,
    ) -> Self {
        Self::builder_for_all(manager, service).build()
    }

    #[cfg(feature = "client")]
    pub fn for_client(manager: ServiceManager) -> Self {
        super::Builder::client(manager).build()
    }

    #[cfg(feature = "server")]
    pub fn for_server(
        service: impl daemon_slayer_server::Service + 'static,
        display_name: String,
        description: String,
    ) -> Self {
        super::Builder::server(service, display_name, description).build()
    }

    #[cfg(all(feature = "client", feature = "server"))]
    pub fn builder_for_all(
        manager: ServiceManager,
        service: impl daemon_slayer_server::Service + 'static,
    ) -> super::Builder {
        super::Builder::new(manager, service)
    }

    #[cfg(feature = "client")]
    pub fn builder_for_client(manager: ServiceManager) -> super::Builder {
        super::Builder::client(manager)
    }

    #[cfg(feature = "server")]
    pub fn builder_for_server(
        service: impl daemon_slayer_server::Service + 'static,
        display_name: String,
        description: String,
    ) -> super::Builder {
        super::Builder::server(service, display_name, description)
    }

    pub(crate) fn from_builder(builder: super::Builder) -> Self {
        let mut cli = Self { builder };
        cli.build_cmd();
        cli
    }

    fn matches(&self, m: &ArgMatches, cmd: &Command, cmd_name: &'static str) -> bool {
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

    pub(crate) fn build_cmd(&mut self) {
        let mut cmd = self
            .builder
            .clap_command
            .clone()
            .name(&self.builder.display_name)
            .about(&self.builder.description);
        for (name, command) in self.builder.commands.iter() {
            let mut hide = false;
            #[cfg(feature = "server")]
            {
                hide = (*name) == ServiceCommands::RUN;
            }

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
                            .help(help_text.as_ref().unwrap())
                            .hide(hide),
                    )
                }
                Command::Subcommand { name, help_text } => {
                    cmd = cmd.subcommand(clap::Command::new(name).about(help_text).hide(hide))
                }
                Command::Default => {}
            }
        }

        self.builder.clap_command = cmd;
    }

    pub async fn handle_input(self) -> Result<InputState, Box<dyn Error + Send + Sync>> {
        let matches = self.builder.clap_command.clone().get_matches();

        if self.handle_cmd(&matches).await? {
            Ok(InputState::Handled)
        } else {
            Ok(InputState::Unhandled(matches))
        }
    }

    #[maybe_async_cfg::only_if(async)]
    async fn handle_cmd(
        mut self,
        matches: &ArgMatches,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        get_handlers!(self, matches,
            #[cfg(feature="console")]
            ServiceCommands::CONSOLE => {
                let mut console = daemon_slayer_console::Console::new(self.builder.manager.unwrap());
                if let Some(health_check) = self.builder.health_check {
                    console.add_health_check(health_check);
                }
                console.run().await?;
                return Ok(true);
            }
            ServiceCommands::HEALTH => {
                if let Some(health_check) = &mut self.builder.health_check {
                    match health_check.invoke().await {
                        Ok(_) => println!("healthy"),
                        Err(_) => println!("unhealthy")
                    }
                }
            }
        );

        #[cfg(feature = "server")]
        for (name, cmd) in self.builder.commands.iter() {
            if self.matches(matches, cmd, name) {
                match *name {
                    ServiceCommands::RUN => {
                        info!("running...");
                        if let Some(service) = self.builder.service {
                            service.run_service_main().await?;
                        }
                        return Ok(true);
                    }

                    #[cfg(feature = "direct")]
                    ServiceCommands::DIRECT => {
                        info!("running...");
                        if let Some(service) = self.builder.service {
                            service.run_service_direct().await?;
                        }
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    #[maybe_async_cfg::only_if(sync)]
    async fn handle_cmd(
        mut self,
        matches: &ArgMatches,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        get_handlers!(self, matches,);

        #[cfg(feature = "server")]
        for (name, cmd) in self.builder.commands.iter() {
            if self.matches(matches, cmd, name) {
                match *name {
                    ServiceCommands::RUN => {
                        info!("running...");
                        if let Some(service) = self.builder.service {
                            service.run_service_main()?;
                        }
                        return Ok(true);
                    }

                    #[cfg(feature = "direct")]
                    ServiceCommands::DIRECT => {
                        info!("running...");
                        if let Some(service) = self.builder.service {
                            service.run_service_direct()?;
                        }
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    pub fn action_type(&self) -> Action {
        let matches = self.builder.clap_command.clone().get_matches();

        for (name, cmd) in self.builder.commands.iter() {
            if self.matches(&matches, cmd, name) {
                #[cfg(feature = "client")]
                {
                    match *name {
                        ServiceCommands::INSTALL
                        | ServiceCommands::UNINSTALL
                        | ServiceCommands::INFO
                        | ServiceCommands::START
                        | ServiceCommands::STOP
                        | ServiceCommands::RESTART
                        | ServiceCommands::PID
                        | ServiceCommands::ENABLE
                        | ServiceCommands::HEALTH
                        | ServiceCommands::DISABLE => {
                            return Action::Client;
                        }
                        #[cfg(feature = "console")]
                        ServiceCommands::CONSOLE => return Action::Client,
                        _ => {}
                    }
                }
                #[cfg(feature = "server")]
                {
                    match *name {
                        ServiceCommands::RUN => {
                            return Action::Server;
                        }
                        #[cfg(feature = "direct")]
                        ServiceCommands::DIRECT => return Action::Server,
                        _ => {}
                    }
                }
                return Action::Unknown;
            }
        }
        Action::Unknown
    }
}
