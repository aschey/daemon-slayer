use crate::{commands::Commands, service_commands::ServiceCommands, Command};

#[maybe_async_cfg::maybe(
    idents(Service, HealthCheck),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
pub struct Builder {
    pub(crate) commands: Commands,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) clap_command: clap::Command,
    #[cfg(feature = "client")]
    pub(crate) manager: Option<daemon_slayer_client::ServiceManager>,
    #[cfg(feature = "server")]
    pub(crate) service: Option<Box<dyn daemon_slayer_server::Service>>,
    #[cfg(feature = "client")]
    pub(crate) health_check: Option<Box<dyn daemon_slayer_client::HealthCheck + Send + 'static>>,
}

#[maybe_async_cfg::maybe(
    idents(Service, HealthCheck, Cli),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
impl Builder {
    #[cfg(all(feature = "client", feature = "server"))]
    pub fn new(
        manager: daemon_slayer_client::ServiceManager,
        service: impl daemon_slayer_server::Service + 'static,
    ) -> Self {
        use daemon_slayer_client::Manager;

        Self {
            commands: Commands::default(),
            display_name: manager.display_name().to_string(),
            description: manager.description().to_string(),
            clap_command: clap::Command::default(),
            health_check: None,
            manager: Some(manager),
            service: Some(Box::new(service)),
        }
    }

    #[cfg(feature = "client")]
    pub fn client(manager: daemon_slayer_client::ServiceManager) -> Self {
        use daemon_slayer_client::Manager;

        Self {
            commands: Commands::default(),
            display_name: manager.display_name().to_string(),
            description: manager.description().to_string(),
            clap_command: clap::Command::default(),
            health_check: None,
            manager: Some(manager),
            #[cfg(feature = "server")]
            service: None,
        }
    }

    #[cfg(feature = "server")]
    pub fn server(
        service: impl daemon_slayer_server::Service + 'static,
        display_name: String,
        description: String,
    ) -> Self {
        Self {
            commands: Commands::default(),
            display_name,
            description,
            clap_command: clap::Command::default(),
            #[cfg(feature = "client")]
            health_check: None,
            #[cfg(feature = "client")]
            manager: None,
            service: Some(Box::new(service)),
        }
    }

    pub fn build(self) -> super::Cli {
        super::Cli::from_builder(self)
    }

    pub fn with_base_command(mut self, command: clap::Command) -> Self {
        self.clap_command = command;
        self
    }

    #[cfg(feature = "client")]
    pub fn with_install_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::INSTALL, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_uninstall_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::UNINSTALL, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_start_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::START, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_stop_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::STOP, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_restart_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::RESTART, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_info_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::INFO, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_pid_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::PID, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_enable_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::ENABLE, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_disable_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::DISABLE, command);
        self
    }

    #[cfg(all(feature = "client", feature = "console"))]
    pub fn with_console_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::CONSOLE, command);
        self
    }

    #[cfg(feature = "client")]
    pub fn with_health_check(
        mut self,
        health_check: Box<dyn daemon_slayer_client::HealthCheck + Send + 'static>,
    ) -> Self {
        self.health_check = Some(health_check);
        if !self.commands.contains_key(ServiceCommands::HEALTH) {
            self.commands.insert(
                ServiceCommands::HEALTH,
                Command::Subcommand {
                    name: ServiceCommands::HEALTH.to_owned(),
                    help_text: "Check the health of the service".to_owned(),
                },
            );
        }

        self
    }

    #[cfg(all(feature = "client", feature = "console"))]
    pub fn with_health_check_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::HEALTH, command);
        self
    }

    #[cfg(all(feature = "server", feature = "direct"))]
    pub fn with_direct_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::DIRECT, command);
        self
    }

    #[cfg(feature = "server")]
    pub fn with_run_command(mut self, command: Command) -> Self {
        self.commands.insert(ServiceCommands::RUN, command);
        self
    }
}
