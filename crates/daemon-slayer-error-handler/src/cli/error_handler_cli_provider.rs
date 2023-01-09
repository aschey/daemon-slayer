use color_eyre::config::Theme;
use daemon_slayer_core::{
    async_trait,
    cli::{clap, Action, CommandConfig, CommandMatch, CommandOutput},
    BoxedError,
};

use crate::ErrorHandler;

#[derive(Default, Clone)]
pub struct ErrorHandlerCliProvider {}

#[async_trait]
impl daemon_slayer_core::cli::CommandProvider for ErrorHandlerCliProvider {
    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![]
    }

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<(), BoxedError> {
        if let Some(Some(action)) = matched_command.as_ref().map(|c| &c.matched_command.action) {
            if action == &Action::Run {
                ErrorHandler::default()
                    .with_theme(Theme::default())
                    .with_write_to_stdout(false)
                    .with_write_to_stderr(false)
                    .with_log(true)
                    .install()?;
                return Ok(());
            }
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
