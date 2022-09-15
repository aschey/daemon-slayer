use std::{collections::HashMap, ops::Deref};

use super::{command::Command, service_commands::ServiceCommands};

pub(crate) struct Commands(HashMap<&'static str, Command>);

impl Commands {
    pub(crate) fn new(enable_client: bool, enable_server: bool) -> Self {
        let mut commands = HashMap::new();
        #[cfg(feature = "client")]
        if enable_client {
            commands.insert(
                ServiceCommands::INSTALL,
                Command::Subcommand {
                    name: ServiceCommands::INSTALL.to_owned(),
                    help_text: "Install the service using the system's service manager".to_owned(),
                },
            );
            commands.insert(
                ServiceCommands::UNINSTALL,
                Command::Subcommand {
                    name: ServiceCommands::UNINSTALL.to_owned(),
                    help_text: "Uninstall the service from the system's service manager".to_owned(),
                },
            );
            commands.insert(
                ServiceCommands::START,
                Command::Subcommand {
                    name: ServiceCommands::START.to_owned(),
                    help_text: "Start the service".to_owned(),
                },
            );
            commands.insert(
                ServiceCommands::INFO,
                Command::Subcommand {
                    name: ServiceCommands::INFO.to_owned(),
                    help_text: "Get the service's current status".to_owned(),
                },
            );
            commands.insert(
                ServiceCommands::PID,
                Command::Subcommand {
                    name: ServiceCommands::PID.to_owned(),
                    help_text: "Get the service's current PID".to_owned(),
                },
            );
            commands.insert(
                ServiceCommands::STOP,
                Command::Subcommand {
                    name: ServiceCommands::STOP.to_owned(),
                    help_text: "Stop the service".to_owned(),
                },
            );

            commands.insert(
                ServiceCommands::RESTART,
                Command::Subcommand {
                    name: ServiceCommands::RESTART.to_owned(),
                    help_text: "Restart the service".to_owned(),
                },
            );

            commands.insert(
                ServiceCommands::ENABLE,
                Command::Subcommand {
                    name: ServiceCommands::ENABLE.to_owned(),
                    help_text: "Enable autostart".to_owned(),
                },
            );

            commands.insert(
                ServiceCommands::DISABLE,
                Command::Subcommand {
                    name: ServiceCommands::DISABLE.to_owned(),
                    help_text: "Disable autostart".to_owned(),
                },
            );

            #[cfg(feature = "console")]
            commands.insert(
                ServiceCommands::CONSOLE,
                Command::Subcommand {
                    name: ServiceCommands::CONSOLE.to_owned(),
                    help_text: "View service console".to_owned(),
                },
            );
        }

        #[cfg(feature = "server")]
        if enable_server {
            commands.insert(
                ServiceCommands::RUN,
                Command::Subcommand {
                    name: ServiceCommands::RUN.to_owned(),
                    help_text: "".to_owned(),
                },
            );

            #[cfg(feature = "direct")]
            commands.insert(ServiceCommands::DIRECT, Command::Default);
        }

        Self(commands)
    }

    pub(crate) fn insert(&mut self, key: &'static str, value: Command) {
        self.0.insert(key, value);
    }

    pub(crate) fn remove(&mut self, key: &'static str) {
        self.0.remove(key);
    }
}

impl Deref for Commands {
    type Target = HashMap<&'static str, Command>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
