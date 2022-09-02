use std::{collections::HashMap, ops::Deref};

use super::{command::Command, service_commands::ServiceCommands};

pub(crate) struct Commands(HashMap<&'static str, Command>);

impl Commands {
    pub(crate) fn insert(&mut self, key: &'static str, value: Command) {
        self.0.insert(key, value);
    }
}

impl Default for Commands {
    fn default() -> Self {
        let mut commands = HashMap::new();
        #[cfg(feature = "client")]
        {
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
                ServiceCommands::STATUS,
                Command::Subcommand {
                    name: ServiceCommands::STATUS.to_owned(),
                    help_text: "Get the service's current status".to_owned(),
                },
            );
            commands.insert(
                ServiceCommands::STOP,
                Command::Subcommand {
                    name: ServiceCommands::STOP.to_owned(),
                    help_text: "Stop the service".to_owned(),
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
        {
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
}

impl Deref for Commands {
    type Target = HashMap<&'static str, Command>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
