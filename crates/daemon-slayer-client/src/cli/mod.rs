use std::future::Future;
use std::io;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use daemon_slayer_core::BoxedError;
use daemon_slayer_core::cli::clap::{self, FromArgMatches, Subcommand};
use daemon_slayer_core::cli::{
    Action, ActionType, ClientAction, CommandMatch, CommandOutput, CommandProvider,
};
use owo_colors::OwoColorize;
use spinoff::Spinner;
pub use spinoff::{Color, spinners};
use tokio::process::Command;
use tokio::time::sleep;

use crate::{ServiceManager, State, Status};

#[derive(Clone, Debug)]
pub struct ClientCliProvider {
    manager: ServiceManager,
    spinner_type: spinners::SpinnerFrames,
    spinner_color: Color,
    matched_command: Option<CliCommands>,
}

struct SpinnerHandle(Spinner);

impl Drop for SpinnerHandle {
    fn drop(&mut self) {
        self.0.clear();
    }
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
    Status {
        #[arg(long)]
        native: bool,
    },
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

    fn get_spinner(&self, message: &str) -> SpinnerHandle {
        #[cfg(windows)]
        colored::control::set_virtual_terminal(true).unwrap();

        SpinnerHandle(Spinner::new(
            self.spinner_type.clone(),
            message.dimmed().to_string(),
            self.spinner_color,
        ))
    }

    async fn wait_for_condition<Fut>(
        &self,
        task: Fut,
        condition: impl Fn(&Status) -> bool,
        wait_message: &str,
        failure_message: &str,
    ) -> io::Result<CommandOutput>
    where
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        println!();
        let mut _sp = self.get_spinner(wait_message);
        let cmd = task.await;
        if let Err(e) = cmd {
            return Ok(CommandOutput::handled(e.red().to_string()));
        }

        // State changes can be asynchronous, wait for the desired state
        // Starting a service can take a while on certain platforms so we'll be conservative with
        // the timeout here
        let max_attempts = 10;
        for _ in 0..max_attempts {
            let info = self.manager.status().await?;

            if condition(&info) {
                return Ok(CommandOutput::handled(info.pretty_print()));
            }
            sleep(Duration::from_secs(1)).await;
        }
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
                CliCommands::Status { .. } => ClientAction::Status,
                CliCommands::Info => ClientAction::Info,
                CliCommands::Pid => ClientAction::Pid,
                CliCommands::Reload => ClientAction::Reload,
                CliCommands::Enable => ClientAction::Enable,
                CliCommands::Disable => ClientAction::Disable,
            })),
        })
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        if let Some(matched_command) = &self.matched_command {
            let state = self.manager.status().await?.state;
            if state == State::NotInstalled
                && !matches!(
                    matched_command,
                    CliCommands::Install | CliCommands::Status { .. }
                )
            {
                return Ok(CommandOutput::handled(
                    "Cannot complete action because service is not installed"
                        .red()
                        .to_string(),
                ));
            }

            if state != State::NotInstalled && matches!(matched_command, CliCommands::Install) {
                return Ok(CommandOutput::handled(
                    "Cannot complete action because service is already installed"
                        .red()
                        .to_string(),
                ));
            }
            let mut manager = self.manager.clone();
            match matched_command {
                CliCommands::Install => {
                    #[cfg(windows)]
                    {
                        if self.manager.config().service_level == crate::config::Level::User {
                            self.wait_for_condition(
                                async move { manager.install().await },
                                |_| true,
                                "Installing...",
                                "Failed to install",
                            )
                            .await?;

                            return Ok(CommandOutput::handled(
                                "Please log out to complete service installation".to_owned(),
                            ));
                        }
                    }

                    return Ok(self
                        .wait_for_condition(
                            async move { manager.install().await },
                            |info| info.state != State::NotInstalled,
                            "Installing...",
                            "Failed to install",
                        )
                        .await?);
                }
                CliCommands::Uninstall => {
                    return Ok(self
                        .wait_for_condition(
                            async move { manager.uninstall().await },
                            |info| info.state == State::NotInstalled,
                            "Uninstalling...",
                            "Failed to uninstall",
                        )
                        .await?);
                }
                CliCommands::Status { native: true } => {
                    let status_command = self.manager.status_command().await?;
                    Command::new(status_command.program)
                        .args(status_command.args)
                        .stdin(Stdio::inherit())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .spawn()?
                        .wait()
                        .await?;
                }
                CliCommands::Status { native: false } => {
                    let _sp = self.get_spinner("Loading...");
                    let status = self.manager.status().await?;
                    return Ok(CommandOutput::handled(status.pretty_print()));
                }
                CliCommands::Info => {
                    let _sp = self.get_spinner("Loading...");
                    let config = self.manager.config();
                    return Ok(CommandOutput::handled(config.pretty_print()));
                }
                CliCommands::Start => {
                    return Ok(self
                        .wait_for_condition(
                            async move { manager.start().await },
                            |info| info.state == State::Started || info.state == State::Listening,
                            "Starting...",
                            "Failed to start",
                        )
                        .await?);
                }
                CliCommands::Stop => {
                    return Ok(self
                        .wait_for_condition(
                            async move { manager.stop().await },
                            |info| info.state == State::Stopped,
                            "Stopping...",
                            "Failed to stop",
                        )
                        .await?);
                }
                CliCommands::Restart => {
                    return Ok(self
                        .wait_for_condition(
                            async move { manager.restart().await },
                            |info| info.state == State::Started || info.state == State::Listening,
                            "Restarting...",
                            "Failed to restart",
                        )
                        .await?);
                }
                CliCommands::Reload => {
                    let _sp = self.get_spinner("Reloading...");
                    self.manager.reload_config().await?;
                    return Ok(CommandOutput::handled("Reloaded".to_string()));
                }
                CliCommands::Enable => {
                    return Ok(self
                        .wait_for_condition(
                            async move { manager.enable_autostart().await },
                            |info| info.autostart == Some(true),
                            "Enabling autostart...",
                            "Failed to enable autostart",
                        )
                        .await?);
                }
                CliCommands::Disable => {
                    return Ok(self
                        .wait_for_condition(
                            async move { manager.disable_autostart().await },
                            |info| info.autostart == Some(false),
                            "Disabling autostart...",
                            "Failed to disable autostart",
                        )
                        .await?);
                }
                CliCommands::Pid => {
                    let _sp = self.get_spinner("Loading...");
                    let pid = self.manager.status().await?.pid;
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
