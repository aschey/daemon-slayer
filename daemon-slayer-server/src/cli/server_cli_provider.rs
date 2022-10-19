use daemon_slayer_core::cli::{
    clap, ActionType, ArgMatchesExt, CommandExt, CommandType, InputState,
};
use std::{collections::HashMap, hash::Hash, marker::PhantomData};
use strum_macros::Display;

#[derive(Display, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ServerAction {
    Run,
    Direct,
}

pub struct ServerCliProvider<S: crate::Service + Send + Sync> {
    commands: HashMap<ServerAction, CommandType>,
    _phantom: PhantomData<S>,
}

impl<S: crate::Service + Send + Sync> Default for ServerCliProvider<S> {
    fn default() -> Self {
        let mut commands = HashMap::default();
        commands.insert(
            ServerAction::Run,
            CommandType::Subcommand {
                name: "run".to_owned(),
                help_text: "".to_owned(),
                hide: true,
            },
        );

        commands.insert(ServerAction::Direct, CommandType::Default);

        Self {
            commands,
            _phantom: Default::default(),
        }
    }
}

impl<S: crate::Service + Send + Sync> ServerCliProvider<S> {
    pub fn with_action(
        mut self,
        action: ServerAction,
        command_type: impl Into<Option<CommandType>>,
    ) -> Self {
        match command_type.into() {
            Some(command_type) => {
                self.commands.insert(action, command_type);
            }
            None => {
                self.commands.remove(&action);
            }
        }
        self
    }
}
#[async_trait::async_trait]
impl<S: crate::Service + Send + Sync> daemon_slayer_core::cli::CommandProvider
    for ServerCliProvider<S>
{
    fn get_action_type(&self) -> ActionType {
        ActionType::Server
    }

    fn get_commands(&self) -> Vec<&CommandType> {
        self.commands.values().collect()
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &daemon_slayer_core::cli::clap::ArgMatches,
    ) -> daemon_slayer_core::cli::InputState {
        for (name, command_type) in &self.commands {
            if matches.matches(command_type) {
                match name {
                    ServerAction::Direct => {
                        S::run_service_direct().await.unwrap();
                    }
                    ServerAction::Run => {
                        S::run_service_main().await.unwrap();
                    }
                }
                return InputState::Handled;
            }
        }
        InputState::Unhandled
    }
}
