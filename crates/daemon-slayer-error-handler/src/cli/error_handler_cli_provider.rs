use color_eyre::config::Theme;
use daemon_slayer_core::cli::{
    clap, Action, ActionType, ArgMatchesExt, CommandConfig, CommandExt, CommandType, InputState,
};
use std::{collections::HashMap, hash::Hash, marker::PhantomData, rc::Rc};
use tracing::Subscriber;

use crate::ErrorHandler;

#[derive(Default, Clone)]
pub struct ErrorHandlerCliProvider {}

#[async_trait::async_trait]
impl daemon_slayer_core::cli::CommandProvider for ErrorHandlerCliProvider {
    fn get_action_type(&self) -> ActionType {
        ActionType::Unknown
    }

    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![]
    }

    fn initialize(&mut self, _matches: &clap::ArgMatches, matched_command: &Option<CommandConfig>) {
        if let Some(Some(action)) = matched_command.as_ref().map(|c| &c.action) {
            if action == &Action::Run {
                ErrorHandler::default()
                    .with_theme(Theme::default())
                    .with_write_to_stdout(false)
                    .with_write_to_stderr(false)
                    .with_log(true)
                    .install()
                    .unwrap();
                return;
            }
        }

        ErrorHandler::default().install().unwrap();
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        _matched_command: &Option<CommandConfig>,
    ) -> InputState {
        InputState::Unhandled
    }
}
