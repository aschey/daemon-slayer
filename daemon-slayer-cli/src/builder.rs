use std::marker::PhantomData;

use crate::{commands::Commands, service_commands::ServiceCommands, Command};
#[cfg(feature = "client")]
use daemon_slayer_client::{Manager, ServiceManager};
#[cfg(feature = "server")]
use daemon_slayer_server::{Handler, Service};

macro_rules! impl_builder {
    () => {
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
        pub fn with_status_command(mut self, command: Command) -> Self {
            self.commands.insert(ServiceCommands::STATUS, command);
            self
        }

        #[cfg(all(feature = "server", feature = "direct"))]
        pub fn with_direct_command(mut self, command: Command) -> Self {
            self.commands.insert(ServiceCommands::DIRECT, command);
            self
        }

        #[cfg(all(feature = "client", feature = "console"))]
        pub fn with_console_command(mut self, command: Command) -> Self {
            self.commands.insert(ServiceCommands::CONSOLE, command);
            self
        }

        #[cfg(feature = "server")]
        pub fn with_run_command(mut self, command: Command) -> Self {
            self.commands.insert(ServiceCommands::RUN, command);
            self
        }
    };
}

#[cfg(feature = "server")]
pub struct Builder<H>
where
    H: Service + Handler,
{
    #[cfg(feature = "client")]
    pub(crate) manager: Option<ServiceManager>,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) commands: Commands,
    _phantom: PhantomData<H>,
}

#[cfg(feature = "server")]
impl<H> Builder<H>
where
    H: Service + Handler,
{
    #[cfg(feature = "client")]
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

    pub(crate) fn new(display_name: String, description: String, commands: Commands) -> Self {
        Self {
            display_name,
            description,
            #[cfg(feature = "client")]
            manager: None,
            commands,
            _phantom: PhantomData::default(),
        }
    }

    impl_builder!();

    #[cfg(feature = "server")]
    pub fn build(self) -> crate::Cli<H> {
        crate::Cli::<H>::from_builder(self)
    }
}

#[cfg(all(not(feature = "server"), feature = "client"))]
pub struct Builder {
    #[cfg(feature = "client")]
    pub(crate) manager: Option<ServiceManager>,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) commands: Commands,
}

#[cfg(all(not(feature = "server"), feature = "client"))]
impl Builder {
    pub(crate) fn from_manager(manager: ServiceManager, commands: Commands) -> Self {
        let display_name = manager.display_name().to_owned();
        let description = manager.description().to_owned();
        Self {
            display_name,
            description,
            manager: Some(manager),
            commands,
        }
    }

    pub(crate) fn new(display_name: String, description: String, commands: Commands) -> Self {
        Self {
            display_name,
            description,
            #[cfg(feature = "client")]
            manager: None,
            commands,
        }
    }

    impl_builder!();

    pub fn build(self) -> crate::Cli {
        crate::Cli::from_builder(self)
    }
}
