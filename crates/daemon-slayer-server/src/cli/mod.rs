use std::marker::PhantomData;

use daemon_slayer_core::cli::clap::parser::ValueSource;
use daemon_slayer_core::cli::clap::{self, ArgAction};
use daemon_slayer_core::cli::{
    Action, ActionType, CommandMatch, CommandOutput, CommandProvider, ServerAction,
};
use daemon_slayer_core::{async_trait, BoxedError, CommandArg};

use crate::Service;

const RUN_ID: &str = "run";
const LABEL_ID: &str = "label";

#[derive(Clone, Debug)]
enum ServerCommand {
    Run,
    Direct,
    Label,
}

#[derive(Clone, Debug)]
pub struct ServerCliProvider<S: Service> {
    input_data: Option<S::InputData>,
    run_command: CommandArg,
    matched_command: Option<ServerCommand>,
    _phantom: PhantomData<S>,
}

impl<S: Service> ServerCliProvider<S> {
    pub fn new(run_command: &CommandArg) -> Self {
        Self {
            run_command: run_command.to_owned(),
            input_data: Default::default(),
            _phantom: Default::default(),
            matched_command: None,
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
        let cmd = match &self.run_command {
            CommandArg::Subcommand(sub) => cmd.subcommand(clap::Command::new(sub)),
            CommandArg::ShortArg(arg) => cmd.arg(
                clap::Arg::new(RUN_ID)
                    .short(*arg)
                    .action(ArgAction::SetTrue),
            ),
            CommandArg::LongArg(arg) => {
                cmd.arg(clap::Arg::new(RUN_ID).long(arg).action(ArgAction::SetTrue))
            }
        };
        cmd.arg(
            clap::Arg::new(LABEL_ID)
                .long(LABEL_ID)
                .action(ArgAction::SetTrue),
        )
    }

    fn matches(&mut self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let has_flags = matches
            .ids()
            .any(|i| matches.value_source(i.as_str()) != Some(ValueSource::DefaultValue));

        match &self.run_command {
            CommandArg::Subcommand(sub)
                if matches!(
                    matches.subcommand(), Some((sub_name, _)) if sub_name == sub) =>
            {
                self.matched_command = Some(ServerCommand::Run);
                Some(CommandMatch {
                    action_type: ActionType::Server,
                    action: Some(Action::Server(ServerAction::Run)),
                })
            }
            CommandArg::LongArg(_) | CommandArg::ShortArg(_) if matches.get_flag(RUN_ID) => {
                self.matched_command = Some(ServerCommand::Run);
                Some(CommandMatch {
                    action_type: ActionType::Server,
                    action: Some(Action::Server(ServerAction::Run)),
                })
            }
            _ if matches.get_flag(LABEL_ID) => {
                self.matched_command = Some(ServerCommand::Label);
                Some(CommandMatch {
                    action_type: ActionType::Other,
                    action: None,
                })
            }
            _ if matches.subcommand().is_none() && !has_flags => {
                self.matched_command = Some(ServerCommand::Direct);
                Some(CommandMatch {
                    action_type: ActionType::Server,
                    action: Some(Action::Server(ServerAction::Direct)),
                })
            }
            _ => None,
        }
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        match self.matched_command {
            Some(ServerCommand::Direct) => {
                S::run_directly(self.input_data).await?;
                Ok(CommandOutput::handled(None))
            }
            Some(ServerCommand::Run) => {
                S::run_as_service(self.input_data).await?;
                Ok(CommandOutput::handled(None))
            }
            Some(ServerCommand::Label) => Ok(CommandOutput::handled(S::label().qualified_name())),
            None => Ok(CommandOutput::unhandled()),
        }
    }
}
