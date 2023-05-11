use std::{io, thread::sleep, time::Duration};

use crate::{Info, ServiceManager, State};
use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, FromArgMatches, Subcommand},
        Action, ActionType, ClientAction, CommandMatch, CommandOutput, CommandProvider,
    },
    BoxedError,
};
use owo_colors::OwoColorize;

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

    fn wait_for_condition(
        &self,
        condition: impl Fn(&Info) -> bool,
        failure_msg: &str,
    ) -> Result<CommandOutput, io::Error> {
        // State changes can be asynchronous, wait for the desired state
        let max_attempts = 5;
        for _ in 0..max_attempts {
            let info = self.manager.info()?;
            if condition(&info) {
                return Ok(CommandOutput::handled(info.pretty_print()));
            }
            println!("{}", "Waiting for desired state...\n".dimmed());
            sleep(Duration::from_secs(1));
        }
        Ok(CommandOutput::handled(failure_msg.red().to_string()))
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
            let state = self.manager.info()?.state;
            if state == State::NotInstalled
                && *action != ClientAction::Install
                && *action != ClientAction::Uninstall
                && *action != ClientAction::Info
            {
                return Ok(CommandOutput::handled(
                    "Cannot complete action because service is not installed"
                        .red()
                        .to_string(),
                ));
            }
            match action {
                ClientAction::Install => {
                    self.manager.install()?;
                    return Ok(self.wait_for_condition(
                        |info| info.state != State::NotInstalled,
                        "Failed to install service",
                    )?);
                }
                ClientAction::Uninstall => {
                    self.manager.uninstall()?;
                    return Ok(self.wait_for_condition(
                        |info| info.state == State::NotInstalled,
                        "Failed to uninstall service",
                    )?);
                }
                ClientAction::Info => {
                    let info = self.manager.info()?;
                    return Ok(CommandOutput::handled(info.pretty_print()));
                }
                ClientAction::Start => {
                    self.manager.start()?;
                    return Ok(self.wait_for_condition(
                        |info| info.state == State::Started,
                        "Failed to start service",
                    )?);
                }
                ClientAction::Stop => {
                    self.manager.stop()?;
                    return Ok(self.wait_for_condition(
                        |info| info.state == State::Stopped,
                        "Failed to stop service",
                    )?);
                }
                ClientAction::Restart => {
                    self.manager.restart()?;
                    return Ok(self.wait_for_condition(
                        |info| info.state == State::Started,
                        "Failed to restart service",
                    )?);
                }
                ClientAction::Reload => self.manager.reload_config()?,
                ClientAction::Enable => {
                    self.manager.enable_autostart()?;
                    return Ok(self.wait_for_condition(
                        |info| info.autostart == Some(true),
                        "Failed to enable autostart",
                    )?);
                }
                ClientAction::Disable => {
                    self.manager.disable_autostart()?;
                    return Ok(self.wait_for_condition(
                        |info| info.autostart == Some(false),
                        "Failed to disable autostart",
                    )?);
                }
                ClientAction::Pid => {
                    let pid = self.manager.info()?.pid;
                    return Ok(CommandOutput::handled(
                        pid.map(|p| p.to_string())
                            .unwrap_or_else(|| "Not running".to_owned()),
                    ));
                }
            }

            return Ok(CommandOutput::handled(None));
        }

        Ok(CommandOutput::unhandled())
    }
}
