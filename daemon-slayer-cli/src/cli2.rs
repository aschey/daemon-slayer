use std::collections::HashMap;

use clap::parser::ValueSource;
use strum_macros::{Display, EnumString};

use crate::{commands::Commands, ActionType, Command, InputState, ServiceCommand};

pub struct Builder {
    pub(crate) commands: HashMap<String, Command>,
    pub(crate) base_command: clap::Command,
    pub(crate) providers: Vec<Box<dyn CommandProvider>>,
}

pub trait CommandProvider {
    fn handle_input(&self, matches: &clap::ArgMatches) -> InputState;

    fn update_command(&self, command: clap::Command) -> clap::Command;

    fn action_type(&self, matches: &clap::ArgMatches) -> ActionType;
}

impl Builder {
    pub fn add_provider(mut self, provider: impl CommandProvider) {
        self.providers.push(Box::new(provider));
    }
}

pub struct Cli {
    pub(crate) providers: Vec<Box<dyn CommandProvider>>,
}

impl Cli {
    pub fn action_type(&self, matches: &clap::ArgMatches) -> ActionType {
        for provider in &self.providers {
            let action_type = provider.action_type(matches);
            if action_type != ActionType::Unknown {
                return action_type;
            }
        }
        ActionType::Unknown
    }

    pub async fn handle_input(&self, matches: &clap::ArgMatches) -> InputState {
        for provider in &self.providers {
            if provider.handle_input(matches) == InputState::Handled {
                return InputState::Handled;
            }
        }
        InputState::Unhandled
    }
}

#[derive(Debug, Display, PartialEq, Eq)]
enum ServerAction {
    Run,
    Direct,
}

pub struct ServerProviderBuilder {
    commands: HashMap<ServerAction, Command>,
}

pub struct ServerProvider {
    commands: HashMap<ServerAction, Command>,
}

impl ServerProvider {
    fn matches(&self, m: &clap::ArgMatches, cmd: &Command, cmd_name: &str) -> bool {
        match cmd {
            Command::Arg {
                short: _,
                long: _,
                help_text: _,
            } => m.value_source(cmd_name) == Some(ValueSource::CommandLine),
            Command::Subcommand {
                name: _,
                help_text: _,
            } => m.subcommand().map(|r| r.0) == Some(cmd_name.clone().into()),
            Command::Default => !m.args_present() && m.subcommand() == None,
        }
    }
}

impl CommandProvider for ServerProvider {
    fn handle_input(&self, matches: &clap::ArgMatches) -> InputState {
        for (action, cmd) in &self.commands {
            if self.matches(matches, cmd, &action.to_string()) {
                match action {
                    ServerAction::Run => todo!(),
                    ServerAction::Direct => todo!(),
                }
            }
        }
        InputState::Unhandled
    }

    fn update_command(&self, command: clap::Command) -> clap::Command {
        //let mut has_default_cmd = false;
        for (action, cmd) in &self.commands {
            let hide = (*action) == ServerAction::Run;
            match cmd {
                Command::Arg {
                    short,
                    long,
                    help_text,
                } => {
                    let mut arg = clap::Arg::new(action.to_string());
                    if let Some(short) = short {
                        arg = arg.short(*short);
                    }
                    if let Some(long) = long {
                        arg = arg.long(long);
                    }

                    command = command.arg(
                        arg.action(clap::ArgAction::SetTrue)
                            .help(help_text.as_ref().unwrap())
                            .hide(hide),
                    )
                }
                Command::Subcommand { name, help_text } => {
                    command =
                        command.subcommand(clap::Command::new(name).about(help_text).hide(hide))
                }
                Command::Default => {
                    //has_default_cmd = true;
                }
            }
        }
        // if !has_default_cmd && self.builder.show_help_if_no_default {
        //     cmd = cmd.arg_required_else_help(true);
        // }
        command
    }

    fn action_type(&self, matches: &clap::ArgMatches) -> ActionType {
        for (action, cmd) in &self.commands {
            if self.matches(matches, cmd, &action.to_string()) {
                return ActionType::Server;
            }
        }
        ActionType::Unknown
    }
}
