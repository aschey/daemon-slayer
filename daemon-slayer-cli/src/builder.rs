use std::collections::HashMap;

use daemon_slayer_core::cli::{Action, CommandExt, CommandProvider, CommandType};

use crate::Cli;

pub struct Builder {
    pub(crate) base_command: clap::Command,
    pub(crate) commands: HashMap<Action, CommandType>,
    pub(crate) providers: Vec<Box<dyn CommandProvider>>,
}

impl Default for Builder {
    fn default() -> Self {
        let base_command = clap::Command::default().arg_required_else_help(true);
        Self {
            base_command,
            providers: Default::default(),
            commands: Default::default(),
        }
    }
}

impl Builder {
    pub fn with_base_command(mut self, command: clap::Command) -> Self {
        self.base_command = command;
        self
    }

    pub fn with_provider(mut self, provider: impl CommandProvider + 'static) -> Self {
        self.providers.push(Box::new(provider));
        self
    }

    pub fn with_default_client_commands(mut self) -> Self {
        self.commands.insert(
            Action::Install,
            CommandType::Subcommand {
                name: Action::Install.to_string(),
                help_text: "Install the service using the system's service manager".to_owned(),
                hide: false,
            },
        );
        self.commands.insert(
            Action::Uninstall,
            CommandType::Subcommand {
                name: Action::Uninstall.to_string(),
                help_text: "Uninstall the service from the system's service manager".to_owned(),
                hide: false,
            },
        );
        self.commands.insert(
            Action::Start,
            CommandType::Subcommand {
                name: Action::Start.to_string(),
                help_text: "Start the service".to_owned(),
                hide: false,
            },
        );
        self.commands.insert(
            Action::Info,
            CommandType::Subcommand {
                name: Action::Info.to_string(),
                help_text: "Get the service's current status".into(),
                hide: false,
            },
        );
        self.commands.insert(
            Action::Pid,
            CommandType::Subcommand {
                name: Action::Pid.to_string(),
                help_text: "Get the service's current PID".to_owned(),
                hide: false,
            },
        );
        self.commands.insert(
            Action::Stop,
            CommandType::Subcommand {
                name: Action::Stop.to_string(),
                help_text: "Stop the service".to_owned(),
                hide: false,
            },
        );

        self.commands.insert(
            Action::Restart,
            CommandType::Subcommand {
                name: Action::Restart.to_string(),
                help_text: "Restart the service".to_owned(),
                hide: false,
            },
        );

        self.commands.insert(
            Action::Enable,
            CommandType::Subcommand {
                name: Action::Enable.to_string(),
                help_text: "Enable autostart".to_owned(),
                hide: false,
            },
        );

        self.commands.insert(
            Action::Disable,
            CommandType::Subcommand {
                name: Action::Disable.to_string(),
                help_text: "Disable autostart".to_owned(),
                hide: false,
            },
        );

        self
    }

    pub fn with_default_server_commands(mut self) -> Self {
        self.commands.insert(
            Action::Run,
            CommandType::Subcommand {
                name: "run".to_owned(),
                help_text: "".to_owned(),
                hide: true,
            },
        );

        self.commands.insert(Action::Direct, CommandType::Default);
        self
    }

    pub fn with_action(
        mut self,
        action: Action,
        command_type: impl Into<Option<CommandType>>,
    ) -> Self {
        if let Some(command_type) = command_type.into() {
            self.commands.insert(action, command_type);
        } else {
            self.commands.remove(&action);
        }

        self
    }

    pub fn build(mut self) -> Cli {
        let mut command = self.base_command;
        for command_type in self.commands.values() {
            command = command.add_command_handler(command_type);
        }
        for provider in &mut self.providers {
            provider.set_base_commands(self.commands.clone());
            command = provider.update_command(command);
        }

        Cli::new(self.providers, command.get_matches())
    }
}
