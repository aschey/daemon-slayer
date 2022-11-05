use crate::Service;
use daemon_slayer_core::cli::{
    clap, Action, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState,
};
use std::{collections::HashMap, hash::Hash, marker::PhantomData};

pub struct ServerCliProvider<S: Service + Send + Sync> {
    commands: HashMap<Action, CommandType>,
    _phantom: PhantomData<S>,
}

impl<S: Service + Send + Sync> Default for ServerCliProvider<S> {
    fn default() -> Self {
        Self {
            commands: Default::default(),
            _phantom: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl<S: Service + Send + Sync> daemon_slayer_core::cli::CommandProvider for ServerCliProvider<S> {
    fn get_action_type(&self) -> ActionType {
        ActionType::Server
    }

    fn set_base_commands(&mut self, commands: HashMap<Action, CommandType>) {
        self.commands = commands;
    }

    fn get_commands(&self) -> Vec<&CommandType> {
        vec![]
    }

    async fn handle_input(mut self: Box<Self>, matches: &clap::ArgMatches) -> InputState {
        for (name, command_type) in &self.commands {
            if name.action_type() == ActionType::Server && matches.matches(command_type) {
                match name {
                    Action::Direct => {
                        S::run_service_direct().await.unwrap();
                    }
                    Action::Run => {
                        S::run_service_main().await.unwrap();
                    }
                    _ => {}
                }
                return InputState::Handled;
            }
        }
        InputState::Unhandled
    }
}
