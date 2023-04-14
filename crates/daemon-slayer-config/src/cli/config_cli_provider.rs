use crate::{AppConfig, ConfigLoadError, Configurable};
use daemon_slayer_core::cli::clap::{Args, FromArgMatches, Subcommand};
use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, ArgMatches},
        ActionType, CommandMatch, CommandOutput, CommandProvider,
    },
    config::ConfigWatcher,
    BoxedError,
};
use tap::TapFallible;
use tracing::error;

#[derive(Subcommand)]
enum ConfigCommands {
    Path,
    Edit,
    Validate,
}

#[derive(Args)]
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

#[derive(Subcommand)]
enum CliCommands {
    Config(ConfigArgs),
}

#[derive(Clone)]
pub struct ConfigCliProvider<T: Configurable> {
    config: AppConfig<T>,
    watchers: Vec<Box<dyn ConfigWatcher>>,
}

impl<T: Configurable> ConfigCliProvider<T> {
    pub fn new(config: AppConfig<T>) -> Self {
        Self {
            config,
            watchers: vec![],
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

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        CliCommands::from_arg_matches(matches).ok()?;
        Some(CommandMatch {
            action_type: ActionType::Client,
            action: None,
        })
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        if let Ok(CliCommands::Config(cmd)) = CliCommands::from_arg_matches(matches) {
            return Ok(match cmd.command {
                Some(ConfigCommands::Path) => {
                    CommandOutput::handled(self.config.full_path().to_string_lossy().to_string())
                }
                Some(ConfigCommands::Edit) => {
                    self.config.edit()?;
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
                        if cmd.plain {
                            CommandOutput::handled(self.config.contents()?)
                        } else {
                            self.config
                                .pretty_print(crate::PrettyPrintOptions { color: cmd.color })?;
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
