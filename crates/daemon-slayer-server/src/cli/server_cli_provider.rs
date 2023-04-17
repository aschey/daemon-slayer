use crate::Service;
use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, ArgAction, parser::ValueSource},
        Action, ActionType, CommandMatch, CommandOutput, CommandProvider, ServerAction,
    },
    BoxedError, CommandArg,
};
use std::marker::PhantomData;

pub struct ServerCliProvider<S: Service> {
    input_data: Option<S::InputData>,
    run_command: CommandArg,
    _phantom: PhantomData<S>,
}

impl<S: Service> ServerCliProvider<S> {
    pub fn new(run_command: &CommandArg) -> Self {
        Self {
            run_command: run_command.to_owned(),
            input_data: Default::default(),
            _phantom: Default::default(),
        }
    }

    pub fn set_input_data(&mut self, input_data: S::InputData) {
        self.input_data = Some(input_data);
    }
}

#[async_trait]
impl<S: Service> CommandProvider for ServerCliProvider<S> {
    fn get_commands(&self, cmd: clap::Command) -> clap::Command {
        let cmd = cmd.arg_required_else_help(false);
        match &self.run_command {
            CommandArg::Subcommand(sub) => cmd.subcommand(clap::Command::new(sub)),
            CommandArg::ShortArg(arg) => cmd.arg(clap::Arg::new("run").short(*arg).action(ArgAction::SetTrue)),
            CommandArg::LongArg(arg) => cmd.arg(clap::Arg::new("run").long(arg).action(ArgAction::SetTrue))
        }
    }

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let has_flags = matches.ids().any(|i| matches.value_source(i.as_str()) != Some(ValueSource::DefaultValue));
        match &self.run_command {
            CommandArg::Subcommand(sub) if matches!(matches.subcommand(), Some((sub_name, _)) 
            if sub_name == sub) => {
                Some(CommandMatch {
                    action_type: ActionType::Server,
                    action: Some(Action::Server(ServerAction::Run)),
                })
            }
            CommandArg::LongArg(_) | CommandArg::ShortArg(_) if matches.get_flag("run") => {
                Some(CommandMatch {
                    action_type: ActionType::Server,
                    action: Some(Action::Server(ServerAction::Run)),
                })
            }
            _ if matches.subcommand().is_none() && !has_flags => Some(CommandMatch {
                action_type: ActionType::Server,
                action: Some(Action::Server(ServerAction::Direct)),
            }),
            _ => None,
        }
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        match matched_command {
            Some(CommandMatch {
                action: Some(Action::Server(ServerAction::Direct)),
                ..
            }) => {
                S::run_directly(self.input_data).await?;
                Ok(CommandOutput::handled(None))
            }
            Some(CommandMatch {
                action: Some(Action::Server(ServerAction::Run)),
                ..
            }) => {
                S::run_as_service(self.input_data).await?;
                Ok(CommandOutput::handled(None))
            }
            _ => Ok(CommandOutput::unhandled()),
        }
    }
}
