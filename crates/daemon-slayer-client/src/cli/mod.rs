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
use spinoff::Spinner;
use std::{io, process::Stdio, time::Duration};
use tokio::{process::Command, time::sleep};

pub use spinoff::spinners;
pub use spinoff::Color;

#[derive(Clone, Debug)]
pub struct ClientCliProvider {
    manager: ServiceManager,
    spinner_type: spinners::SpinnerFrames,
    spinner_color: Color,
    matched_command: Option<CliCommands>,
}

#[derive(Subcommand, PartialEq, Eq, Clone, Debug)]
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
    /// Get the service status from the native service manager
    Status,
}

impl ClientCliProvider {
    pub fn new(manager: ServiceManager) -> Self {
        Self {
            manager,
            spinner_type: spinners::Dots.into(),
            spinner_color: Color::Cyan,
            matched_command: None,
        }
    }

    pub fn with_spinner_type(self, spinner_type: impl Into<spinners::SpinnerFrames>) -> Self {
        Self {
            spinner_type: spinner_type.into(),
            ..self
        }
    }

    pub fn with_spinner_color(self, spinner_color: Color) -> Self {
        Self {
            spinner_color,
            ..self
        }
    }

    async fn wait_for_condition(
        &self,
        condition: impl Fn(&Info) -> bool,
        wait_message: &str,
        failure_message: &str,
    ) -> Result<CommandOutput, io::Error> {
        #[cfg(windows)]
        colored::control::set_virtual_terminal(true).unwrap();

        println!();
        let sp = Spinner::new(
            self.spinner_type.clone(),
            wait_message.dimmed().to_string(),
            self.spinner_color,
        );

        // State changes can be asynchronous, wait for the desired state
        // Starting a service can take a while on certain platforms so we'll be conservative with the timeout here
        let max_attempts = 10;
        for _ in 0..max_attempts {
            let info = self.manager.info().await?;
            if condition(&info) {
                sp.stop();
                println!();
                return Ok(CommandOutput::handled(info.pretty_print()));
            }
            sleep(Duration::from_secs(1)).await;
        }
        sp.stop();
        println!();
        Ok(CommandOutput::handled(failure_message.red().to_string()))
    }
}

#[async_trait]
impl CommandProvider for ClientCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        CliCommands::augment_subcommands(command)
    }

    fn matches(&mut self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let cmd = CliCommands::from_arg_matches(matches).ok()?;
        self.matched_command = Some(cmd.clone());
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
                CliCommands::Status => ClientAction::Status,
            })),
        })
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        if let Some(matched_command) = &self.matched_command {
            let state = self.manager.info().await?.state;
            if state == State::NotInstalled
                && *matched_command != CliCommands::Install
                && *matched_command != CliCommands::Uninstall
                && *matched_command != CliCommands::Info
            {
                return Ok(CommandOutput::handled(
                    "Cannot complete action because service is not installed"
                        .red()
                        .to_string(),
                ));
            }

            match matched_command {
                CliCommands::Install => {
                    self.manager.install().await?;

                    #[cfg(windows)]
                    if self.manager.config().service_level == crate::config::Level::User {
                        return Ok(CommandOutput::handled(
                            "Please log out to complete service installation".to_owned(),
                        ));
                    }

                    return Ok(self
                        .wait_for_condition(
                            |info| info.state != State::NotInstalled,
                            "Installing...",
                            "Failed to install",
                        )
                        .await?);
                }
                CliCommands::Uninstall => {
                    self.manager.uninstall().await?;
                    return Ok(self
                        .wait_for_condition(
                            |info| info.state == State::NotInstalled,
                            "Uninstalling...",
                            "Failed to uninstall",
                        )
                        .await?);
                }
                CliCommands::Info => {
                    let info = self.manager.info().await?;
                    return Ok(CommandOutput::handled(info.pretty_print()));
                }
                CliCommands::Start => {
                    self.manager.start().await?;
                    return Ok(self
                        .wait_for_condition(
                            |info| info.state == State::Started,
                            "Starting...",
                            "Failed to start",
                        )
                        .await?);
                }
                CliCommands::Stop => {
                    self.manager.stop().await?;
                    return Ok(self
                        .wait_for_condition(
                            |info| info.state == State::Stopped,
                            "Stopping...",
                            "Failed to stop",
                        )
                        .await?);
                }
                CliCommands::Restart => {
                    self.manager.restart().await?;
                    return Ok(self
                        .wait_for_condition(
                            |info| info.state == State::Started,
                            "Restarting...",
                            "Failed to restart",
                        )
                        .await?);
                }
                CliCommands::Reload => self.manager.reload_config().await?,
                CliCommands::Enable => {
                    self.manager.enable_autostart().await?;
                    return Ok(self
                        .wait_for_condition(
                            |info| info.autostart == Some(true),
                            "Enabling autostart...",
                            "Failed to enable autostart",
                        )
                        .await?);
                }
                CliCommands::Disable => {
                    self.manager.disable_autostart().await?;
                    return Ok(self
                        .wait_for_condition(
                            |info| info.autostart == Some(false),
                            "Disabling autostart...",
                            "Failed to disable autostart",
                        )
                        .await?);
                }
                CliCommands::Pid => {
                    let pid = self.manager.info().await?.pid;
                    return Ok(CommandOutput::handled(
                        pid.map(|p| p.to_string())
                            .unwrap_or_else(|| "Not running".to_owned()),
                    ));
                }
                CliCommands::Status => {
                    let status_command = self.manager.status_command();
                    Command::new(status_command.program)
                        .args(status_command.args)
                        .stdin(Stdio::inherit())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .spawn()?
                        .wait()
                        .await?;
                }
            }

            return Ok(CommandOutput::handled(None));
        }

        Ok(CommandOutput::unhandled())
    }
}
