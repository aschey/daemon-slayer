use color_eyre::config::Theme;
use daemon_slayer_core::{
    async_trait,
    cli::{clap, Action, CommandMatch, CommandOutput, ServerAction},
    BoxedError,
};

use crate::ErrorHandler;

#[derive(Default, Clone)]
pub struct ErrorHandlerCliProvider {}

#[async_trait]
impl daemon_slayer_core::cli::CommandProvider for ErrorHandlerCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn matches(&self, _matches: &clap::ArgMatches) -> Option<CommandMatch> {
        None
    }

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<(), BoxedError> {
        if let Some(CommandMatch {
            action: Some(Action::Server(ServerAction::Run)),
            ..
        }) = matched_command
        {
            ErrorHandler::default()
                .with_theme(Theme::default())
                .with_write_to_stdout(false)
                .with_write_to_stderr(false)
                .with_log(true)
                .install()?;
            return Ok(());
        }

        ErrorHandler::default().install()?;
        Ok(())
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        Ok(CommandOutput::unhandled())
    }
}
