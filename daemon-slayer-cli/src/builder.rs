use std::collections::HashMap;

use daemon_slayer_core::cli::{
    Action, ActionType, CommandConfig, CommandExt, CommandProvider, CommandType,
};

use crate::Cli;

pub struct Builder {
    pub(crate) base_command: clap::Command,
    pub(crate) providers: Vec<Box<dyn CommandProvider>>,
}

impl Default for Builder {
    fn default() -> Self {
        let base_command = clap::Command::default().arg_required_else_help(true);
        Self {
            base_command,
            providers: Default::default(),
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

    pub fn initialize(mut self) -> Cli {
        let mut command = self.base_command;

        for provider in &mut self.providers {
            command = provider.update_command(command);
        }

        Cli::new(self.providers, command.get_matches())
    }
}
