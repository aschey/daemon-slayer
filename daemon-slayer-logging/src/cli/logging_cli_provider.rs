use crate::{LoggerBuilder, LoggerGuard};
use daemon_slayer_core::cli::{
    clap, Action, ActionType, ArgMatchesExt, CommandConfig, CommandExt, CommandType, InputState,
};
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, util::SubscriberInitExt};

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
    ) -> (
        impl SubscriberInitExt + Subscriber + for<'a> LookupSpan<'a>,
        LoggerGuard,
    ) {
        self.builder
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .build()
            .unwrap()
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

    fn initialize(&mut self, _matches: &clap::ArgMatches, matched_command: &Option<CommandConfig>) {
        let mut builder = self.builder.lock().unwrap();
        if let Some(current_builder) = builder.take() {
            match matched_command
                .as_ref()
                .map(|c| (&c.action, &c.action_type))
            {
                Some((action, ActionType::Client)) => {
                    if action == &Some(Action::Install) {
                        current_builder.register().unwrap();
                    } else if action == &Some(Action::Uninstall) {
                        current_builder.deregister().unwrap();
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
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        _matched_command: &Option<CommandConfig>,
    ) -> InputState {
        InputState::Unhandled
    }
}
