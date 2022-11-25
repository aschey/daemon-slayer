use crate::{LoggerBuilder, LoggerGuard};
use daemon_slayer_core::cli::{
    clap, Action, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState,
};
use std::{collections::HashMap, hash::Hash, marker::PhantomData, rc::Rc};
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, util::SubscriberInitExt};

#[derive(Clone)]
pub struct LoggingCliProvider {
    builder: LoggerBuilder,
    commands: HashMap<Action, CommandType>,
}

impl LoggingCliProvider {
    pub fn new(builder: LoggerBuilder) -> Self {
        Self {
            builder,
            commands: Default::default(),
        }
    }

    pub fn get_logger(
        self,
    ) -> (
        impl SubscriberInitExt + Subscriber + for<'a> LookupSpan<'a>,
        LoggerGuard,
    ) {
        self.builder.build().unwrap()
    }
}

#[async_trait::async_trait]
impl daemon_slayer_core::cli::CommandProvider for LoggingCliProvider {
    fn get_action_type(&self) -> ActionType {
        ActionType::Unknown
    }

    fn get_commands(&self) -> Vec<&CommandType> {
        vec![]
    }

    fn set_base_commands(&mut self, commands: HashMap<Action, CommandType>) {
        self.commands = commands;
    }

    fn initialize(&mut self, matches: &clap::ArgMatches) {
        for (name, command_type) in &self.commands {
            if matches.matches(command_type) {
                match (name, name.action_type()) {
                    (Action::Install, _) => {
                        self.builder.register().unwrap();
                    }
                    (Action::Uninstall, _) => {
                        self.builder.deregister().unwrap();
                    }
                    (_, ActionType::Client) => {
                        self.builder = self.builder.clone().with_log_to_stderr(false);
                    }
                    _ => {}
                }

                return;
            }
        }
    }

    async fn handle_input(mut self: Box<Self>, _: &clap::ArgMatches) -> InputState {
        InputState::Unhandled
    }
}
