use std::process::Command;

use confique::Config;
use daemon_slayer_core::cli::{
    ActionType, ArgMatchesExt, CommandExt, CommandProvider, CommandType, InputState,
};

use crate::AppConfig;

pub struct ConfigCliProvider<T: Config + Send> {
    config_command: CommandType,
    config: AppConfig<T>,
}

impl<T: Config + Send> ConfigCliProvider<T> {
    pub fn new(config: AppConfig<T>) -> Self {
        Self {
            config,
            config_command: CommandType::Subcommand {
                name: "config".to_owned(),
                help_text: "view and edit config".to_owned(),
                hide: false,
                children: Some(vec![
                    CommandType::Subcommand {
                        name: "path".to_owned(),
                        help_text: "show the config file path".to_owned(),
                        hide: false,
                        children: None,
                    },
                    CommandType::Subcommand {
                        name: "edit".to_owned(),
                        help_text: "open the config file using the system text editor".to_owned(),
                        hide: false,
                        children: None,
                    },
                    CommandType::Subcommand {
                        name: "view".to_owned(),
                        help_text: "show the config file contents".to_owned(),
                        hide: false,
                        children: Some(vec![CommandType::Arg {
                            id: "no_color".to_owned(),
                            short: None,
                            long: Some("no-color".to_owned()),
                            help_text: Some("disable colors".to_owned()),
                            hide: false,
                        }]),
                    },
                ]),
            },
        }
    }
}
#[async_trait::async_trait]
impl<T: Config + Send> CommandProvider for ConfigCliProvider<T> {
    fn get_action_type(&self) -> ActionType {
        ActionType::Unknown
    }

    fn get_commands(&self) -> Vec<&CommandType> {
        vec![&self.config_command]
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &daemon_slayer_core::cli::clap::ArgMatches,
    ) -> InputState {
        if matches.matches(&self.config_command) {
            let (_, sub) = matches.subcommand().unwrap();
            if let CommandType::Subcommand {
                name: _,
                help_text: _,
                hide: _,
                children,
            } = self.config_command
            {
                for arg in children.unwrap().iter() {
                    if sub.matches(arg) {
                        if let CommandType::Subcommand {
                            name,
                            help_text: _,
                            hide: _,
                            children,
                        } = arg
                        {
                            match name.as_str() {
                                "path" => {
                                    println!("{}", self.config.path().to_string_lossy())
                                }
                                "edit" => {
                                    self.config.edit();
                                }
                                "view" => {
                                    self.config.pretty_print();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        InputState::Unhandled
    }
}
