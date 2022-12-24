use crate::{AppConfig, ConfigLoadError, Configurable};
use daemon_slayer_core::{
    cli::{
        clap::{self, ArgMatches},
        ActionType, ArgMatchesExt, CommandConfig, CommandMatch, CommandProvider, CommandType,
        InputState,
    },
    BoxedError,
};

#[derive(Clone)]
pub struct ConfigCliProvider<T: Configurable> {
    config_command: CommandConfig,
    config: AppConfig<T>,
    #[cfg(feature = "client")]
    manager: Option<Box<dyn daemon_slayer_client::Manager>>,
}

impl<T: Configurable> ConfigCliProvider<T> {
    pub fn new(config: AppConfig<T>) -> Self {
        Self {
            #[cfg(feature = "client")]
            manager: None,
            config,
            config_command: CommandConfig {
                action_type: ActionType::Client,
                action: None,
                command_type: CommandType::Subcommand {
                    name: "config".to_owned(),
                    help_text: "view and edit config".to_owned(),
                    hide: false,
                    children: vec![
                        CommandType::Subcommand {
                            name: "path".to_owned(),
                            help_text: "show the config file path".to_owned(),
                            hide: false,
                            children: vec![],
                        },
                        CommandType::Subcommand {
                            name: "edit".to_owned(),
                            help_text: "open the config file using the system text editor"
                                .to_owned(),
                            hide: false,
                            children: vec![],
                        },
                        #[cfg(feature = "pretty-print")]
                        CommandType::Arg {
                            id: "plain".to_owned(),
                            short: Some('p'),
                            long: Some("plain".to_owned()),
                            help_text: Some("print in plain text".to_owned()),
                            hide: false,
                        },
                        #[cfg(feature = "pretty-print")]
                        CommandType::Arg {
                            id: "no_color".to_owned(),
                            short: None,
                            long: Some("no-color".to_owned()),
                            help_text: Some("disable colors".to_owned()),
                            hide: false,
                        },
                        CommandType::Subcommand {
                            name: "validate".to_owned(),
                            help_text: "validate the config file".to_owned(),
                            hide: false,
                            children: vec![],
                        },
                    ],
                },
            },
        }
    }

    #[cfg(feature = "client")]
    pub fn with_manager(mut self, manager: Box<dyn daemon_slayer_client::Manager>) -> Self {
        self.manager = Some(manager);
        self
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
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<InputState, BoxedError> {
        match matched_command
            .as_ref()
            .map(|c| (&c.matched_command.command_type, &c.matches))
        {
            Some((CommandType::Subcommand { name, .. }, sub)) if name == "config" => {
                if let CommandType::Subcommand { children, .. } = &self.config_command.command_type
                {
                    #[cfg_attr(not(feature = "client"), allow(unused))]
                    let (state, name) = find_subcommand(sub, children, &self.config)?;
                    #[cfg(feature = "client")]
                    if name == Some("edit") {
                        if let Some(manager) = &mut self.manager {
                            manager.on_config_changed()?;
                        }
                    }
                    return Ok(state);
                }
            }
            _ => {}
        }

        Ok(InputState::Unhandled)
    }
}

fn find_subcommand<'a, T: Configurable>(
    sub: &ArgMatches,
    children: &'a [CommandType],
    config: &AppConfig<T>,
) -> Result<(InputState, Option<&'a str>), BoxedError> {
    for arg in children.iter() {
        if let (CommandType::Subcommand { name, .. }, Some(sub)) = (arg, sub.matches(arg)) {
            let input_state = handle_config_subcommand(Some(name.as_str()), &sub, config);
            return input_state.map(|state| (state, Some(name.as_str())));
        }
    }
    return handle_config_subcommand(None, sub, config).map(|state| (state, None));
}

fn handle_config_subcommand<T: Configurable>(
    name: Option<&str>,
    #[cfg_attr(not(feature = "pretty-print"), allow(unused))] sub: &ArgMatches,
    config: &AppConfig<T>,
) -> Result<InputState, BoxedError> {
    match name {
        Some("path") => {
            println!("{}", config.full_path().to_string_lossy());
            return Ok(InputState::Handled);
        }
        Some("edit") => {
            config.edit()?;
            return Ok(InputState::Handled);
        }
        Some("validate") => {
            match config.read_config() {
                Ok(_) => println!("Valid"),
                Err(ConfigLoadError(_, msg)) => {
                    println!("Invalid: {msg}")
                }
            }

            return Ok(InputState::Handled);
        }
        None => {
            #[cfg(feature = "pretty-print")]
            {
                let plain = *sub.get_one::<bool>("plain").unwrap_or(&false);
                if plain {
                    println!("{}", config.contents()?);
                } else {
                    let no_color = *sub.get_one::<bool>("no_color").unwrap_or(&false);
                    config.pretty_print(crate::PrettyPrintOptions { color: !no_color })?;
                }
            }
            #[cfg(not(feature = "pretty-print"))]
            {
                println!("{}", config.contents()?);
            }
            return Ok(InputState::Handled);
        }
        _ => {}
    }
    Ok(InputState::Unhandled)
}
