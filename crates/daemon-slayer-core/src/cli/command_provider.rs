use std::fmt::Debug;

use downcast_rs::{impl_downcast, Downcast};

use super::{Action, ActionType, CommandOutput};
use crate::BoxedError;

#[derive(Clone, Debug)]
pub struct CommandMatch {
    pub action_type: ActionType,
    pub action: Option<Action>,
}

impl_downcast!(CommandProvider);

#[async_trait::async_trait]
pub trait CommandProvider: Downcast + Send + 'static {
    fn get_commands(&self, cmd: clap::Command) -> clap::Command;

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch>;

    async fn handle_input(
        self: Box<Self>,
        matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError>;

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<(), BoxedError> {
        Ok(())
    }
}
