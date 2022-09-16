use std::{collections::HashMap, ops::Deref};

use super::{command::Command, service_command::ServiceCommand};

pub(crate) struct Commands(HashMap<ServiceCommand, Command>);

impl Commands {
    pub(crate) fn new(enable_client: bool, enable_server: bool) -> Self {
        let mut commands = HashMap::new();
        #[cfg(feature = "client")]
        if enable_client {
            commands.insert(
                ServiceCommand::Install,
                Command::Subcommand {
                    name: ServiceCommand::Install.into(),
                    help_text: "Install the service using the system's service manager".to_owned(),
                },
            );
            commands.insert(
                ServiceCommand::Uninstall,
                Command::Subcommand {
                    name: ServiceCommand::Uninstall.into(),
                    help_text: "Uninstall the service from the system's service manager".to_owned(),
                },
            );
            commands.insert(
                ServiceCommand::Start,
                Command::Subcommand {
                    name: ServiceCommand::Start.into(),
                    help_text: "Start the service".to_owned(),
                },
            );
            commands.insert(
                ServiceCommand::Info,
                Command::Subcommand {
                    name: ServiceCommand::Info.into(),
                    help_text: "Get the service's current status".into(),
                },
            );
            commands.insert(
                ServiceCommand::Pid,
                Command::Subcommand {
                    name: ServiceCommand::Pid.into(),
                    help_text: "Get the service's current PID".to_owned(),
                },
            );
            commands.insert(
                ServiceCommand::Stop,
                Command::Subcommand {
                    name: ServiceCommand::Stop.into(),
                    help_text: "Stop the service".to_owned(),
                },
            );

            commands.insert(
                ServiceCommand::Restart,
                Command::Subcommand {
                    name: ServiceCommand::Restart.into(),
                    help_text: "Restart the service".to_owned(),
                },
            );

            commands.insert(
                ServiceCommand::Enable,
                Command::Subcommand {
                    name: ServiceCommand::Enable.into(),
                    help_text: "Enable autostart".to_owned(),
                },
            );

            commands.insert(
                ServiceCommand::Disable,
                Command::Subcommand {
                    name: ServiceCommand::Disable.into(),
                    help_text: "Disable autostart".to_owned(),
                },
            );

            #[cfg(feature = "console")]
            commands.insert(
                ServiceCommand::Console,
                Command::Subcommand {
                    name: ServiceCommand::Console.into(),
                    help_text: "View service console".to_owned(),
                },
            );
        }

        #[cfg(feature = "server")]
        if enable_server {
            commands.insert(
                ServiceCommand::Run,
                Command::Subcommand {
                    name: ServiceCommand::Run.into(),
                    help_text: "".to_owned(),
                },
            );

            #[cfg(feature = "direct")]
            commands.insert(ServiceCommand::Direct, Command::Default);
        }

        Self(commands)
    }

    pub(crate) fn insert(&mut self, key: ServiceCommand, value: Command) {
        self.0.insert(key, value);
    }

    pub(crate) fn remove(&mut self, key: &ServiceCommand) {
        self.0.remove(key);
    }
}

impl Deref for Commands {
    type Target = HashMap<ServiceCommand, Command>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
