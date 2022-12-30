use crate::Service;
use daemon_slayer_core::{
    cli::{
        clap, Action, ActionType, CommandConfig, CommandMatch, CommandProvider, CommandType,
        InputState,
    },
    BoxedError, CommandArg,
};
use std::{collections::HashMap, marker::PhantomData};

pub struct ServerCliProvider<S: Service> {
    commands: HashMap<Action, CommandConfig>,
    input_data: Option<S::InputData>,
    _phantom: PhantomData<S>,
}

fn to_run_command(argument: &CommandArg) -> CommandType {
    match argument {
        CommandArg::Subcommand(name) => CommandType::Subcommand {
            name: name.to_owned(),
            help_text: "".to_owned(),
            hide: true,
            children: vec![],
        },
        CommandArg::ShortArg(arg) => CommandType::Arg {
            id: "run".to_owned(),
            short: Some(arg.to_owned()),
            long: None,
            help_text: None,
            hide: true,
        },
        CommandArg::LongArg(arg) => CommandType::Arg {
            id: "run".to_owned(),
            short: None,
            long: Some(arg.to_owned()),
            help_text: None,
            hide: true,
        },
    }
}

impl<S: Service> ServerCliProvider<S> {
    pub fn new(run_command: &CommandArg) -> Self {
        let mut commands = HashMap::new();
        commands.insert(
            Action::Run,
            CommandConfig {
                action_type: ActionType::Server,
                action: Some(Action::Run),
                command_type: to_run_command(run_command),
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
    pub fn set_input_data(&mut self, input_data: S::InputData) {
        self.input_data = Some(input_data);
    }
}

#[async_trait::async_trait]
impl<S: Service> CommandProvider for ServerCliProvider<S> {
    fn get_commands(&self) -> Vec<&CommandConfig> {
        self.commands.values().collect()
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<InputState, BoxedError> {
        match matched_command.as_ref().map(|c| &c.matched_command.action) {
            Some(Some(Action::Direct)) => {
                S::run_directly(self.input_data).await?;
                Ok(InputState::Handled)
            }
            Some(Some(Action::Run)) => {
                S::run_as_service(self.input_data).await?;
                Ok(InputState::Handled)
            }
            _ => Ok(InputState::Unhandled),
        }
    }
}
