use std::error::Error;

use crate::{action::ActionType, service_command::ServiceCommand, Action, Command, InputState};
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
                        ServiceCommand::Install => {
                            #[cfg(feature="logging")]
                            {
                                let logger_builder = daemon_slayer_logging::LoggerBuilder::new($self.builder.name);
                                logger_builder.register()?;
                            }

                            manager.install()?;
                            return Ok(true);
                        }
                        ServiceCommand::Uninstall => {
                            manager.uninstall()?;
                            #[cfg(feature="logging")]
                            {
                                let logger_builder = daemon_slayer_logging::LoggerBuilder::new($self.builder.name);
                                logger_builder.deregister()?;
                            }

                            return Ok(true);
                        }
                        ServiceCommand::Info => {
                            println!("{:?}", manager.info()?);
                            return Ok(true);
                        }
                        ServiceCommand::Pid => {
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
                        ServiceCommand::Start => {
                            manager.start()?;
                            return Ok(true);
                        }
                        ServiceCommand::Stop => {
                            manager.stop()?;
                            return Ok(true);
                        }
                        ServiceCommand::Restart => {
                            manager.restart()?;
                            return Ok(true);
                        }
                        ServiceCommand::Enable => {
                            manager.set_autostart_enabled(true)?;
                            return Ok(true);
                        }
                        ServiceCommand::Disable => {
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
        // service: impl daemon_slayer_server::Service + 'static,
        name: String,
        display_name: String,
        description: String,
    ) -> Self {
        super::Builder::server(name, display_name, description).build()
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
        //service: impl daemon_slayer_server::Service + 'static,
        name: String,
        display_name: String,
        description: String,
    ) -> super::Builder {
        super::Builder::server(name, display_name, description)
    }

    pub(crate) fn from_builder(builder: super::Builder) -> Self {
        let mut cli = Self { builder };
        cli.build_cmd();
        cli
    }

    fn matches(&self, m: &ArgMatches, cmd: &Command, cmd_name: &ServiceCommand) -> bool {
        match cmd {
            Command::Arg {
                short: _,
                long: _,
                help_text: _,
            } => m.get_one::<bool>(cmd_name.clone().into()) == Some(&true),
            Command::Subcommand {
                name: _,
                help_text: _,
            } => m.subcommand().map(|r| r.0) == Some(cmd_name.clone().into()),
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

        let mut has_default_cmd = false;
        for (name, command) in self.builder.commands.iter() {
            #[cfg(not(feature = "server"))]
            let mut hide = false;
            #[cfg(feature = "server")]
            let hide = (*name) == ServiceCommand::Run;

            match command {
                Command::Arg {
                    short,
                    long,
                    help_text,
                } => {
                    let mut arg = Arg::new(name.to_string());
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
                Command::Default => {
                    has_default_cmd = true;
                }
            }
        }

        if !has_default_cmd && self.builder.show_help_if_no_default {
            cmd = cmd.arg_required_else_help(true);
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
            ServiceCommand::Console => {
                let mut console = daemon_slayer_console::Console::new(self.builder.manager.unwrap());
                if let Some(health_check) = self.builder.health_check {
                    console.add_health_check(health_check);
                }
                console.run().await?;
                return Ok(true);
            }
            ServiceCommand::Health => {
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
                    ServiceCommand::Run => {
                        // info!("running...");
                        // if let Some(service) = self.builder.service {
                        //     service.run_service_main().await?;
                        // }
                        return Ok(true);
                    }

                    #[cfg(feature = "direct")]
                    ServiceCommand::Direct => {
                        // info!("running...");
                        // if let Some(service) = self.builder.service {
                        //     service.run_service_direct().await?;
                        // }
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
                    ServiceCommand::Run => {
                        // info!("running...");
                        // if let Some(service) = self.builder.service {
                        //     service.run_service_main()?;
                        // }
                        return Ok(true);
                    }

                    #[cfg(feature = "direct")]
                    ServiceCommand::Direct => {
                        // info!("running...");
                        // if let Some(service) = self.builder.service {
                        //     service.run_service_direct()?;
                        // }
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    #[cfg(feature = "logging")]
    pub fn configure_logger(&self) -> daemon_slayer_logging::LoggerBuilder {
        let logger_builder = daemon_slayer_logging::LoggerBuilder::new(&self.builder.name);
        let action = self.action();
        if action.action_type == ActionType::Client {
            logger_builder
                .with_log_to_stderr(false)
                .with_log_to_stdout(false)
        } else {
            logger_builder
                .with_log_to_stderr(true)
                .with_log_to_stdout(false)
        }
    }

    #[cfg(feature = "error-handler")]
    pub fn configure_error_handler(&self) -> daemon_slayer_error_handler::ErrorHandler {
        use daemon_slayer_error_handler::{color_eyre::config::Theme, ErrorHandler, PanicBehavior};

        let action = self.action();

        #[cfg(feature = "server")]
        if action.command == Some(ServiceCommand::Run) {
            ErrorHandler::default()
                .with_theme(Theme::new())
                .with_panic_behavior(PanicBehavior::Log)
        } else {
            ErrorHandler::default()
                .with_panic_behavior(PanicBehavior::Print)
                .with_theme(Theme::dark())
        }

        #[cfg(not(feature = "server"))]
        ErrorHandler::default().with_panic_behavior(PanicBehavior::Print)
    }

    pub fn action(&self) -> Action {
        let matches = self.builder.clap_command.clone().get_matches();

        for (name, cmd) in self.builder.commands.iter() {
            if self.matches(&matches, cmd, name) {
                #[cfg(feature = "client")]
                {
                    match name {
                        ServiceCommand::Install
                        | ServiceCommand::Uninstall
                        | ServiceCommand::Info
                        | ServiceCommand::Start
                        | ServiceCommand::Stop
                        | ServiceCommand::Restart
                        | ServiceCommand::Pid
                        | ServiceCommand::Enable
                        | ServiceCommand::Health
                        | ServiceCommand::Disable => {
                            return Action {
                                action_type: ActionType::Client,
                                command: Some(name.to_owned()),
                            };
                        }
                        #[cfg(feature = "console")]
                        ServiceCommand::Console => {
                            return Action {
                                action_type: ActionType::Client,
                                command: Some(name.to_owned()),
                            }
                        }
                        _ => {}
                    }
                }
                #[cfg(feature = "server")]
                {
                    match *name {
                        ServiceCommand::Run => {
                            return Action {
                                action_type: ActionType::Server,
                                command: Some(name.to_owned()),
                            };
                        }
                        #[cfg(feature = "direct")]
                        ServiceCommand::Direct => {
                            return Action {
                                action_type: ActionType::Server,
                                command: Some(name.to_owned()),
                            }
                        }
                        _ => {}
                    }
                }
                return Action {
                    action_type: ActionType::Unknown,
                    command: None,
                };
            }
        }
        Action {
            action_type: ActionType::Unknown,
            command: None,
        }
    }
}
