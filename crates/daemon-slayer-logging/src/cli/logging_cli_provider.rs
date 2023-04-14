use crate::{LoggerBuilder, LoggerCreationError, ReloadHandle};
use daemon_slayer_core::{
    async_trait,
    cli::{clap, Action, ActionType, ClientAction, CommandMatch, CommandOutput, CommandProvider},
    BoxedError,
};
use std::sync::{Arc, Mutex};
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, util::SubscriberInitExt};

#[derive(thiserror::Error, Debug)]
pub enum LoggerInitializationError {
    #[error("{0}")]
    CreationFailure(#[source] LoggerCreationError),
    #[error("The logger was already created")]
    AlreadyCreated,
}

#[derive(Clone)]
pub struct LoggingCliProvider {
    pub builder: Arc<Mutex<Option<LoggerBuilder>>>,
}

impl LoggingCliProvider {
    pub fn new(builder: LoggerBuilder) -> Self {
        Self {
            builder: Arc::new(Mutex::new(Some(builder))),
        }
    }

    pub fn get_logger(
        self,
    ) -> Result<
        (
            impl SubscriberInitExt + Subscriber + for<'a> LookupSpan<'a>,
            ReloadHandle,
        ),
        LoggerInitializationError,
    > {
        self.builder
            .lock()
            .unwrap()
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

    fn matches(&self, _matches: &clap::ArgMatches) -> Option<CommandMatch> {
        None
    }

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<(), BoxedError> {
        let mut builder = self.builder.lock().unwrap();
        if let Some(current_builder) = builder.take() {
            if let Some(matched) = matched_command {
                *builder = Some(match matched.action_type {
                    ActionType::Client => {
                        if matched.action == Some(Action::Client(ClientAction::Install)) {
                            current_builder.register()?;
                        } else if matched.action == Some(Action::Client(ClientAction::Uninstall)) {
                            current_builder.deregister()?;
                        }
                        current_builder
                            .with_log_to_stderr(false)
                            .with_ipc_logger(false)
                    }
                    ActionType::Server => current_builder.with_ipc_logger(true),
                    ActionType::Unknown => current_builder,
                })
            }
        }
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
