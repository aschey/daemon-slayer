use daemon_slayer_client::{Manager, ServiceManager};
use daemon_slayer_core::cli::{
    clap, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState,
};
use std::{collections::HashMap, hash::Hash, marker::PhantomData};

use crate::Console;

pub struct ConsoleCliProvider {
    command: CommandType,
    manager: ServiceManager,
}

impl ConsoleCliProvider {
    pub fn new(manager: ServiceManager) -> Self {
        Self {
            manager,
            command: CommandType::Subcommand {
                name: "console".to_owned(),
                help_text: "view service console".to_owned(),
                hide: false,
            },
        }
    }
    pub fn with_action(mut self, command_type: CommandType) -> Self {
        self.command = command_type;
        self
    }
}
#[async_trait::async_trait]
impl daemon_slayer_core::cli::CommandProvider for ConsoleCliProvider {
    fn get_action_type(&self) -> ActionType {
        ActionType::Server
    }

    fn get_commands(&self) -> Vec<&CommandType> {
        vec![&self.command]
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &daemon_slayer_core::cli::clap::ArgMatches,
    ) -> daemon_slayer_core::cli::InputState {
        if matches.matches(&self.command) {
            let mut console = Console::new(self.manager);
            console.run().await.unwrap();
            return InputState::Handled;
        }
        InputState::Unhandled
    }
}
