use color_eyre::config::Theme;
use daemon_slayer_core::cli::{clap, Action, CommandMatch, CommandOutput, ServerAction};
use daemon_slayer_core::{async_trait, BoxedError};

use crate::ErrorHandler;

#[derive(Clone)]
pub struct ErrorHandlerCliProvider {
    #[cfg(feature = "notify")]
    notification: Option<
        std::sync::Arc<
            dyn daemon_slayer_core::notify::AsyncNotification<Output = ()> + Send + Sync,
        >,
    >,
}

impl Default for ErrorHandlerCliProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorHandlerCliProvider {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "notify")]
            notification: None,
        }
    }

    #[cfg(feature = "notify")]
    pub fn with_notification<N>(self, notification: N) -> Self
    where
        N: daemon_slayer_core::notify::AsyncNotification<Output = ()> + Send + Sync + 'static,
    {
        Self {
            notification: Some(std::sync::Arc::new(notification)),
        }
    }
}

#[async_trait]
impl daemon_slayer_core::cli::CommandProvider for ErrorHandlerCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        command
    }

    fn matches(&mut self, _matches: &clap::ArgMatches) -> Option<CommandMatch> {
        None
    }

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        matched_command: Option<&CommandMatch>,
    ) -> Result<(), BoxedError> {
        if let Some(CommandMatch {
            action: Some(Action::Server(ServerAction::Run)),
            ..
        }) = matched_command
        {
            #[allow(unused_mut)]
            let mut handler = ErrorHandler::default()
                .with_theme(Theme::default())
                .with_write_to_stdout(false)
                .with_write_to_stderr(false)
                .with_log(true);
            #[cfg(feature = "notify")]
            if let Some(notification) = self.notification.clone() {
                handler = handler.with_dyn_notification(notification);
            }

            handler.install()?;
            return Ok(());
        }

        ErrorHandler::default().install()?;
        Ok(())
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        Ok(CommandOutput::unhandled())
    }
}
