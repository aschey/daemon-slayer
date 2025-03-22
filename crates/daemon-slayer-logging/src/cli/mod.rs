use async_trait::async_trait;
use daemon_slayer_core::BoxedError;
use daemon_slayer_core::cli::{
    Action, ActionType, ClientAction, CommandMatch, CommandOutput, CommandProvider, clap,
};
use daemon_slayer_core::config::Accessor;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

use crate::{LoggerBuilder, LoggerCreationError, ReloadHandle, UserConfig};

#[derive(thiserror::Error, Debug)]
pub enum LoggerInitializationError {
    #[error("{0}")]
    CreationFailure(#[source] LoggerCreationError),
    #[error("The logger was already created")]
    AlreadyCreated,
}

#[derive(Debug)]
pub struct LoggingCliProvider {
    pub builder: Option<LoggerBuilder>,
}

impl LoggingCliProvider {
    pub fn new(builder: LoggerBuilder) -> Self {
        Self {
            builder: Some(builder),
        }
    }

    pub fn get_logger_with_reload<S>(
        mut self,
        service: S,
    ) -> Result<
        (
            impl SubscriberInitExt + Subscriber + for<'a> LookupSpan<'a>,
            ReloadHandle,
        ),
        LoggerInitializationError,
    >
    where
        S: Accessor<UserConfig> + Clone + Unpin + 'static,
    {
        self.builder
            .take()
            .ok_or(LoggerInitializationError::AlreadyCreated)?
            .build_with_reload(service)
            .map_err(LoggerInitializationError::CreationFailure)
    }

    pub fn get_logger(
        mut self,
    ) -> Result<
        impl SubscriberInitExt + Subscriber + for<'a> LookupSpan<'a>,
        LoggerInitializationError,
    > {
        self.builder
            .take()
            .ok_or(LoggerInitializationError::AlreadyCreated)?
            .build()
            .map_err(LoggerInitializationError::CreationFailure)
    }
}

#[async_trait]
impl CommandProvider for LoggingCliProvider {
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
        if let Some(current_builder) = self.builder.take() {
            if let Some(matched) = matched_command {
                self.builder = Some(match matched.action_type {
                    ActionType::Client => {
                        if matched.action == Some(Action::Client(ClientAction::Install)) {
                            current_builder.register()?;
                        } else if matched.action == Some(Action::Client(ClientAction::Uninstall)) {
                            current_builder.deregister()?;
                        }
                        #[cfg(feature = "ipc")]
                        let res = current_builder
                            .with_log_to_stderr(false)
                            .with_ipc_logger(false);
                        #[cfg(not(feature = "ipc"))]
                        let res = current_builder.with_log_to_stderr(false);
                        res
                    }
                    ActionType::Server => {
                        #[cfg(feature = "ipc")]
                        let res = current_builder.with_ipc_logger(true);
                        #[cfg(not(feature = "ipc"))]
                        let res = current_builder;
                        res
                    }
                    ActionType::Unknown | ActionType::Other => current_builder,
                })
            } else {
                self.builder = Some(current_builder);
            }
        }
        Ok(())
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        Ok(CommandOutput::unhandled())
    }
}
