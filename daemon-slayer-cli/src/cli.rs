use daemon_slayer_core::cli::{ActionType, CommandProvider, InputState};

pub struct Builder {
    base_command: clap::Command,
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

    pub fn build(self) -> (Cli, clap::Command) {
        let mut command = self.base_command;
        for provider in &self.providers {
            command = provider.update_command(command);
        }

        (Cli::new(self.providers), command)
    }
}

pub struct Cli {
    pub(crate) providers: Vec<Box<dyn CommandProvider>>,
}

impl Cli {
    pub fn builder() -> Builder {
        Builder::default()
    }

    fn new(providers: Vec<Box<dyn CommandProvider>>) -> Self {
        Self { providers }
    }

    pub fn action_type(&self, matches: &clap::ArgMatches) -> ActionType {
        for provider in &self.providers {
            let action_type = provider.action_type(matches);
            if action_type != ActionType::Unknown {
                return action_type;
            }
        }
        ActionType::Unknown
    }

    pub async fn handle_input(&self, matches: &clap::ArgMatches) -> InputState {
        for provider in &self.providers {
            if provider.handle_input(matches).await == InputState::Handled {
                return InputState::Handled;
            }
        }
        InputState::Unhandled
    }
}
