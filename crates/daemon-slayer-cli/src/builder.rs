use std::ffi::OsString;

use crate::Cli;
use daemon_slayer_core::{cli::CommandProvider, BoxedError};

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
        if self
            .providers
            .iter()
            .any(|p| p.type_id() == provider.type_id())
        {
            panic!(
                "Provider with type {:?} is already registered",
                provider.type_id()
            );
        }
        self.providers.push(Box::new(provider));
        self
    }

    fn build_command(&mut self) -> clap::Command {
        let mut command = self.base_command.clone();

        for provider in &mut self.providers {
            command = provider.get_commands(command);
        }
        command
    }

    pub fn initialize(mut self) -> Result<Cli, BoxedError> {
        let mut command = self.build_command();

        Cli::new(self.providers, command.render_help(), command.get_matches())
    }

    pub fn initialize_from<I, T>(mut self, itr: I) -> Result<Cli, BoxedError>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut command = self.build_command();
        Cli::new(
            self.providers,
            command.render_help(),
            command.get_matches_from(itr),
        )
    }
}
