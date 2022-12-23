use crate::{AppConfig, ConfigLoadError, Configurable};
use daemon_slayer_client::Manager;
use daemon_slayer_core::{
    cli::{
        clap, ActionType, ArgMatchesExt, CommandConfig, CommandProvider, CommandType, InputState,
    },
    BoxedError,
};

#[derive(Clone)]
pub struct ConfigCliProvider<T: Configurable> {
    config_command: CommandConfig,
    config: AppConfig<T>,
    manager: Box<dyn Manager>,
}

impl<T: Configurable> ConfigCliProvider<T> {
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
                        #[cfg(feature = "pretty-print")]
                        CommandType::Subcommand {
                            name: "pretty".to_owned(),
                            help_text: "pretty-print the config file contents".to_owned(),
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
impl<T: Configurable> CommandProvider for ConfigCliProvider<T> {
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
    ) -> Result<InputState, BoxedError> {
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
                                children: _,
                            } = arg
                            {
                                match name.as_str() {
                                    "path" => {
                                        println!("{}", self.config.full_path().to_string_lossy());
                                        return Ok(InputState::Handled);
                                    }
                                    "edit" => {
                                        self.config.edit()?;
                                        self.manager.on_config_changed()?;
                                        return Ok(InputState::Handled);
                                    }
                                    "view" => {
                                        println!("{}", self.config.contents()?);
                                    }
                                    #[cfg(feature = "pretty-print")]
                                    "pretty" => {
                                        let (_, sub) = sub.subcommand().unwrap();
                                        let no_color =
                                            *sub.get_one::<bool>("no_color").unwrap_or(&false);
                                        self.config.pretty_print(crate::PrettyPrintOptions {
                                            color: !no_color,
                                        })?;

                                        return Ok(InputState::Handled);
                                    }
                                    "validate" => {
                                        match self.config.read_config() {
                                            Ok(_) => println!("Valid"),
                                            Err(ConfigLoadError(_, msg)) => {
                                                println!("Invalid: {msg}")
                                            }
                                        }

                                        return Ok(InputState::Handled);
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

        Ok(InputState::Unhandled)
    }
}
