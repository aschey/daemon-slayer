use crate::Service;
use daemon_slayer_core::cli::{clap, Action, ActionType, CommandConfig, CommandType, InputState};
use std::{collections::HashMap, error::Error, marker::PhantomData};

pub struct ServerCliProvider<S: Service + Send + Sync> {
    commands: HashMap<Action, CommandConfig>,
    input_data: Option<S::InputData>,
    _phantom: PhantomData<S>,
}

impl<S: Service + Send + Sync + 'static> Default for ServerCliProvider<S> {
    fn default() -> Self {
        let mut commands = HashMap::new();
        commands.insert(
            Action::Run,
            CommandConfig {
                action_type: ActionType::Server,
                action: Some(Action::Run),
                command_type: CommandType::Subcommand {
                    name: "run".to_owned(),
                    help_text: "".to_owned(),
                    hide: true,
                    children: None,
                },
            },
        );
        commands.insert(
            Action::Direct,
            CommandConfig {
                action_type: ActionType::Server,
                command_type: CommandType::Default,
                action: Some(Action::Direct),
            },
        );
        Self {
            commands,
            input_data: Default::default(),
            _phantom: Default::default(),
        }
    }
}

impl<S: Service + Send + Sync + 'static> ServerCliProvider<S> {
    pub fn set_input_data(&mut self, input_data: S::InputData) {
        self.input_data = Some(input_data);
    }
}

#[async_trait::async_trait]
impl<S: Service + Send + Sync + 'static> daemon_slayer_core::cli::CommandProvider
    for ServerCliProvider<S>
{
    fn get_action_type(&self) -> ActionType {
        ActionType::Server
    }

    fn get_commands(&self) -> Vec<&CommandConfig> {
        self.commands.values().collect()
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandConfig>,
    ) -> Result<InputState, Box<dyn Error>> {
        match matched_command.as_ref().map(|c| &c.action) {
            Some(Some(Action::Direct)) => {
                S::run_directly(self.input_data).await.unwrap();
                Ok(InputState::Handled)
            }
            Some(Some(Action::Run)) => {
                S::run_as_service(self.input_data).await.unwrap();
                Ok(InputState::Handled)
            }
            _ => Ok(InputState::Unhandled),
        }
    }
}
