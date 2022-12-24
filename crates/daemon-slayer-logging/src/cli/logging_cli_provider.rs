use crate::{LoggerBuilder, LoggerCreationError, LoggerGuard};
use daemon_slayer_core::{
    cli::{clap, Action, ActionType, CommandConfig, CommandMatch, InputState},
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
            LoggerGuard,
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

#[async_trait::async_trait]
impl daemon_slayer_core::cli::CommandProvider for LoggingCliProvider {
    fn get_action_type(&self) -> ActionType {
        ActionType::Unknown
    }

    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![]
    }

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<(), BoxedError> {
        let mut builder = self.builder.lock().unwrap();
        if let Some(current_builder) = builder.take() {
            match matched_command
                .as_ref()
                .map(|c| (&c.matched_command.action, &c.matched_command.action_type))
            {
                Some((action, ActionType::Client)) => {
                    if action == &Some(Action::Install) {
                        current_builder.register()?;
                    } else if action == &Some(Action::Uninstall) {
                        current_builder.deregister()?;
                    }
                    *builder = Some(
                        current_builder
                            .with_log_to_stderr(false)
                            .with_ipc_logger(false),
                    );
                }
                Some((_, ActionType::Server)) => {
                    *builder = Some(current_builder.with_ipc_logger(true));
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<InputState, BoxedError> {
        Ok(InputState::Unhandled)
    }
}
