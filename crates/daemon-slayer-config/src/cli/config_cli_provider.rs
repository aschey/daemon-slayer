use std::process::Command;

use confique::Config;
use daemon_slayer_client::Manager;
use daemon_slayer_core::cli::{
    clap, ActionType, ArgMatchesExt, CommandConfig, CommandExt, CommandProvider, CommandType,
    InputState,
};

use crate::AppConfig;

#[derive(Clone)]
pub struct ConfigCliProvider<T: Config + Default + Send + Sync + Clone + 'static> {
    config_command: CommandConfig,
    config: AppConfig<T>,
    manager: Box<dyn Manager>,
}

impl<T: Config + Default + Send + Sync + Clone + 'static> ConfigCliProvider<T> {
    pub fn new(config: AppConfig<T>, manager: Box<dyn Manager>) -> Self {
        Self {
            manager,
            config,
            config_command: CommandConfig {
                action_type: ActionType::Client,
                action: None,
                command_type: CommandType::Subcommand {
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
                            help_text: "open the config file using the system text editor"
                                .to_owned(),
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
                        CommandType::Subcommand {
                            name: "validate".to_owned(),
                            help_text: "validate the config file".to_owned(),
                            hide: false,
                            children: None,
                        },
                    ]),
                },
            },
        }
    }
}

#[async_trait::async_trait]
impl<T: Config + Default + Send + Sync + Clone + 'static> CommandProvider for ConfigCliProvider<T> {
    fn get_action_type(&self) -> ActionType {
        ActionType::Client
    }

    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![&self.config_command]
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &clap::ArgMatches,
        matched_command: &Option<CommandConfig>,
    ) -> InputState {
        match matched_command.as_ref().map(|c| &c.command_type) {
            Some(CommandType::Subcommand {
                name,
                help_text: _,
                hide: _,
                children: _,
            }) if name == "config" => {
                let (_, sub) = matches.subcommand().unwrap();
                if let CommandType::Subcommand {
                    name: _,
                    help_text: _,
                    hide: _,
                    children,
                } = self.config_command.command_type
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
                                        println!("{}", self.config.path().to_string_lossy());
                                        return InputState::Handled;
                                    }
                                    "edit" => {
                                        self.config.edit();
                                        self.manager.on_configuration_changed().unwrap();
                                        return InputState::Handled;
                                    }
                                    "view" => {
                                        self.config.pretty_print();
                                        return InputState::Handled;
                                    }
                                    "validate" => {
                                        // TODO: error checking
                                        self.config.read_config();
                                        return InputState::Handled;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        InputState::Unhandled
    }
}
