use color_eyre::config::Theme;
use daemon_slayer_core::{
    async_trait,
    cli::{clap, Action, CommandMatch, CommandOutput, ServerAction},
    BoxedError, Label,
};

use crate::ErrorHandler;

#[derive(Clone, Debug)]
pub struct ErrorHandlerCliProvider {
    label: Label,
}

impl ErrorHandlerCliProvider {
    pub fn new(label: Label) -> Self {
        Self { label }
    }
}

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
            let handler = ErrorHandler::new(self.label.clone())
                .with_theme(Theme::default())
                .with_write_to_stdout(false)
                .with_write_to_stderr(false)
                .with_log(true);
            #[cfg(feature = "notify")]
            let handler = handler.with_notify(true);

            handler.install()?;
            return Ok(());
        }

        ErrorHandler::new(self.label.clone()).install()?;
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
