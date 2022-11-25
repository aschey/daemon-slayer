use color_eyre::config::Theme;
use daemon_slayer_core::cli::{
    clap, Action, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState,
};
use std::{collections::HashMap, hash::Hash, marker::PhantomData, rc::Rc};
use tracing::Subscriber;

use crate::ErrorHandler;

#[derive(Default)]
pub struct ErrorHandlerCliProvider {
    commands: HashMap<Action, CommandType>,
}

#[async_trait::async_trait]
impl daemon_slayer_core::cli::CommandProvider for ErrorHandlerCliProvider {
    fn get_action_type(&self) -> ActionType {
        ActionType::Unknown
    }

    fn get_commands(&self) -> Vec<&CommandType> {
        vec![]
    }

    fn set_base_commands(&mut self, commands: HashMap<Action, CommandType>) {
        self.commands = commands;
    }

    fn initialize(&mut self, matches: &clap::ArgMatches) {
        for (name, command_type) in &self.commands {
            if matches.matches(command_type) {
                if name == &Action::Run {
                    ErrorHandler::default()
                        .with_theme(Theme::default())
                        .with_write_to_stdout(false)
                        .with_write_to_stderr(false)
                        .with_log(true)
                        .install()
                        .unwrap();
                }
                return;
            }
        }
        ErrorHandler::default().install().unwrap();
    }

    async fn handle_input(mut self: Box<Self>, _: &clap::ArgMatches) -> InputState {
        InputState::Unhandled
    }
}
