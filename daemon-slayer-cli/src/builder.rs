use crate::{commands::Commands, service_command::ServiceCommand, Command};

macro_rules! impl_command_builder {
    ($name: ident, $command: ident) => {
        pub fn $name(mut self, command: impl Into<Option<Command>>) -> Self {
            match command.into() {
                Some(command) => {
                    self.commands.insert(ServiceCommand::$command, command);
                }
                None => {
                    self.commands.remove(&ServiceCommand::$command);
                }
            }
            self
        }
    };
}
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
    pub(crate) health_check:
        Option<Box<dyn daemon_slayer_client::health_check::HealthCheck + Send + 'static>>,
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
            commands: Commands::new(true, true),
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
            commands: Commands::new(true, false),
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
            commands: Commands::new(false, true),
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
    impl_command_builder!(with_install_command, Install);

    #[cfg(feature = "client")]
    impl_command_builder!(with_uninstall_command, Uninstall);

    #[cfg(feature = "client")]
    impl_command_builder!(with_start_command, Start);

    #[cfg(feature = "client")]
    impl_command_builder!(with_stop_command, Stop);

    #[cfg(feature = "client")]
    impl_command_builder!(with_restart_command, Restart);

    #[cfg(feature = "client")]
    impl_command_builder!(with_info_command, Info);

    #[cfg(feature = "client")]
    impl_command_builder!(with_pid_command, Pid);

    #[cfg(feature = "client")]
    impl_command_builder!(with_enable_command, Enable);

    #[cfg(feature = "client")]
    impl_command_builder!(with_disable_command, Disable);

    #[cfg(all(feature = "client", feature = "console"))]
    impl_command_builder!(with_console_command, Console);

    #[cfg(all(feature = "client", feature = "console"))]
    impl_command_builder!(with_health_check_command, Health);

    #[cfg(all(feature = "server", feature = "direct"))]
    impl_command_builder!(with_direct_command, Direct);

    #[cfg(feature = "server")]
    impl_command_builder!(with_run_command, Run);

    #[cfg(feature = "client")]
    pub fn with_health_check(
        mut self,
        health_check: Box<dyn daemon_slayer_client::health_check::HealthCheck + Send + 'static>,
    ) -> Self {
        self.health_check = Some(health_check);
        if !self.commands.contains_key(&ServiceCommand::Health) {
            self.commands.insert(
                ServiceCommand::Health,
                Command::Subcommand {
                    name: ServiceCommand::Health.into(),
                    help_text: "Check the health of the service".to_owned(),
                },
            );
        }

        self
    }
}
