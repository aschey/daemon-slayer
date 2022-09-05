maybe_async_cfg::content! {

    #![maybe_async_cfg::default(
        idents(
            ClientCli,
            ServerCli,
            Cli,
            Handler,
            Service
        )
    )]

use std::marker::PhantomData;

use crate::{commands::Commands, service_commands::ServiceCommands, Command};
#[cfg(feature = "client")]
use daemon_slayer_client::{Manager, ServiceManager};


macro_rules! impl_client_builder {
    () => {
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

        #[cfg(feature = "console")]
        pub fn with_console_command(mut self, command: Command) -> Self {
            self.commands.insert(ServiceCommands::CONSOLE, command);
            self
        }
    };
}

macro_rules! impl_server_builder {
    () => {
        #[cfg(feature = "direct")]
        pub fn with_direct_command(mut self, command: Command) -> Self {
            self.commands.insert(ServiceCommands::DIRECT, command);
            self
        }

        pub fn with_run_command(mut self, command: Command) -> Self {
            self.commands.insert(ServiceCommands::RUN, command);
            self
        }
    };
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
#[cfg(feature = "server")]
pub struct ServerCliBuilder<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler,
{
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) commands: Commands,
    _phantom: PhantomData<H>,
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
#[cfg(feature = "server")]
impl<H> ServerCliBuilder<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler,
{
    pub(crate) fn new(display_name: String, description: String, commands: Commands) -> Self {
        Self {
            display_name,
            description,
            commands,
            _phantom: PhantomData::default(),
        }
    }

    impl_server_builder!();

    pub fn build(self) -> crate::server::ServerCli<H> {
        crate::server::ServerCli::<H>::from_builder(self)
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
#[cfg(feature = "client")]
pub struct ClientCliBuilder {
    pub(crate) manager: ServiceManager,
    pub(crate) commands: Commands,
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
#[cfg(feature = "client")]
impl ClientCliBuilder {
    pub(crate) fn from_manager(manager: ServiceManager, commands: Commands) -> Self {
        Self { manager, commands }
    }

    impl_client_builder!();

    pub fn build(self) -> crate::ClientCli {
        crate::ClientCli::from_builder(self)
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
#[cfg(all(feature = "server", feature = "client"))]
pub struct CliBuilder<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler,
{
    #[cfg(feature = "client")]
    pub(crate) manager: Option<ServiceManager>,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) commands: Commands,
    _phantom: PhantomData<H>,
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
#[cfg(all(feature = "server", feature = "client"))]
impl<H> CliBuilder<H>
where
    H: daemon_slayer_server::Service + daemon_slayer_server::Handler ,
{
    pub(crate) fn from_manager(manager: ServiceManager, commands: Commands) -> Self {
        let display_name = manager.display_name().to_owned();
        let description = manager.description().to_owned();
        Self {
            display_name,
            description,
            manager: Some(manager),
            commands,
            _phantom: PhantomData::default(),
        }
    }

    impl_server_builder!();

    impl_client_builder!();

    pub fn build(self) -> crate::combined::Cli<H> {
        crate::combined::Cli::<H>::from_builder(self)
    }
}
}
