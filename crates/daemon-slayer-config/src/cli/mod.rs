use async_trait::async_trait;
use daemon_slayer_core::BoxedError;
use daemon_slayer_core::cli::clap::{
    Args, FromArgMatches, Subcommand, {self},
};
use daemon_slayer_core::cli::{ActionType, CommandMatch, CommandOutput, CommandProvider};
use daemon_slayer_core::config::ConfigWatcher;
use derivative::Derivative;
use tap::TapFallible;
use tracing::error;

use crate::{AppConfig, ConfigLoadError, Configurable};

#[derive(Subcommand, Debug, Clone)]
enum ConfigCommands {
    Path,
    Edit,
    Validate,
}

#[derive(Args, Debug, Clone)]
struct ConfigArgs {
    #[command(subcommand)]
    command: Option<ConfigCommands>,
    #[cfg(feature = "pretty-print")]
    #[arg(short, long)]
    plain: bool,
    #[cfg(feature = "pretty-print")]
    #[arg(short, long)]
    color: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum CliCommands {
    Config(ConfigArgs),
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct ConfigCliProvider<T: Configurable> {
    config: AppConfig<T>,
    #[derivative(Debug = "ignore")]
    watchers: Vec<Box<dyn ConfigWatcher>>,
    matched_args: Option<ConfigArgs>,
}

impl<T: Configurable> ConfigCliProvider<T> {
    pub fn new(config: AppConfig<T>) -> Self {
        Self {
            config,
            watchers: vec![],
            matched_args: None,
        }
    }

    pub fn with_config_watcher(mut self, watcher: impl ConfigWatcher) -> Self {
        self.watchers.push(Box::new(watcher));
        self
    }
}

#[async_trait]
impl<T: Configurable> CommandProvider for ConfigCliProvider<T> {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliCommands::augment_subcommands(command)
    }

    fn matches(&mut self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let command_match = CliCommands::from_arg_matches(matches).ok()?;
        let CliCommands::Config(args) = command_match;
        self.matched_args = Some(args);
        Some(CommandMatch {
            action_type: ActionType::Client,
            action: None,
        })
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        if let Some(args) = &self.matched_args {
            return Ok(match args.command {
                Some(ConfigCommands::Path) => {
                    CommandOutput::handled(self.config.full_path().to_string_lossy().to_string())
                }
                Some(ConfigCommands::Edit) => {
                    self.config.edit()?;
                    for watcher in &mut self.watchers {
                        watcher
                            .on_config_changed()
                            .await
                            .tap_err(|e| error!("Error handling config update: {e:?}"))
                            .ok();
                    }
                    CommandOutput::handled(None)
                }
                Some(ConfigCommands::Validate) => match self.config.read_config() {
                    Ok(_) => CommandOutput::handled("Valid".to_owned()),
                    Err(ConfigLoadError(_, msg)) => {
                        CommandOutput::handled(format!("Invalid: {msg}"))
                    }
                },
                None => {
                    #[cfg(feature = "pretty-print")]
                    {
                        if args.plain {
                            CommandOutput::handled(self.config.contents()?)
                        } else {
                            self.config
                                .pretty_print(crate::PrettyPrintOptions { color: args.color })?;
                            CommandOutput::handled(None)
                        }
                    }
                    #[cfg(not(feature = "pretty-print"))]
                    {
                        CommandOutput::handled(self.config.contents()?)
                    }
                }
            });
        }

        Ok(CommandOutput::unhandled())
    }
}

#[cfg(test)]
#[path = "./config_cli_provider_test.rs"]
mod config_cli_provider_test;
