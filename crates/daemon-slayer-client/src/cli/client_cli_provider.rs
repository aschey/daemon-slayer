use crate::ServiceManager;
use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, FromArgMatches, Subcommand},
        Action, ActionType, ClientAction, CommandMatch, CommandOutput, CommandProvider,
    },
    BoxedError,
};

#[derive(Clone, Debug)]
pub struct ClientCliProvider {
    manager: ServiceManager,
}

#[derive(Subcommand)]
enum CliCommands {
    /// Install the service using the system's service manager
    Install,
    /// Uninstall the service from the system's service manager
    Uninstall,
    /// Start the service
    Start,
    /// Stop the service
    Stop,
    /// Restart the service
    Restart,
    /// Get the service's current status
    Info,
    /// Get the service's current PID
    Pid,
    /// Reload the service config
    Reload,
    /// Enable autostart
    Enable,
    /// Disable autostart
    Disable,
}

impl ClientCliProvider {
    pub fn new(manager: ServiceManager) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl CommandProvider for ClientCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliCommands::augment_subcommands(command)
    }

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let cmd = CliCommands::from_arg_matches(matches).ok()?;
        Some(CommandMatch {
            action_type: ActionType::Client,
            action: Some(Action::Client(match cmd {
                CliCommands::Install => ClientAction::Install,
                CliCommands::Uninstall => ClientAction::Uninstall,
                CliCommands::Start => ClientAction::Start,
                CliCommands::Stop => ClientAction::Stop,
                CliCommands::Restart => ClientAction::Restart,
                CliCommands::Info => ClientAction::Info,
                CliCommands::Pid => ClientAction::Pid,
                CliCommands::Reload => ClientAction::Reload,
                CliCommands::Enable => ClientAction::Enable,
                CliCommands::Disable => ClientAction::Disable,
            })),
        })
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        if let Some(CommandMatch {
            action: Some(Action::Client(action)),
            ..
        }) = matched_command
        {
            match action {
                ClientAction::Install => self.manager.install()?,
                ClientAction::Uninstall => self.manager.uninstall()?,
                ClientAction::Info => {
                    let info = self.manager.info()?;
                    return Ok(CommandOutput::handled(info.pretty_print()));
                }
                ClientAction::Start => self.manager.start()?,
                ClientAction::Stop => self.manager.stop()?,
                ClientAction::Restart => self.manager.restart()?,
                ClientAction::Reload => self.manager.reload_config()?,
                ClientAction::Enable => self.manager.enable_autostart()?,
                ClientAction::Disable => self.manager.disable_autostart()?,
                ClientAction::Pid => {
                    let pid = self.manager.info()?.pid;
                    return Ok(CommandOutput::handled(
                        pid.map(|p| p.to_string())
                            .unwrap_or_else(|| "Not running".to_owned()),
                    ));
                }
                _ => return Ok(CommandOutput::unhandled()),
            }
            return Ok(CommandOutput::handled(None));
        }

        Ok(CommandOutput::unhandled())
    }
}
