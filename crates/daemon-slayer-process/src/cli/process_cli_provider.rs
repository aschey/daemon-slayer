use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self},
        ActionType, CommandConfig, CommandMatch, CommandOutput, CommandProvider, CommandType,
    },
    BoxedError,
};

use crate::ProcessManager;

pub struct ProcessCliProvider {
    command: CommandConfig,
    pid: Option<u32>,
}

impl ProcessCliProvider {
    pub fn new(pid: Option<u32>) -> Self {
        Self {
            pid,
            command: CommandConfig {
                action_type: ActionType::Client,
                command_type: CommandType::Subcommand {
                    name: "process".to_owned(),
                    help_text: "View and control the service process".to_owned(),
                    hide: false,
                    children: vec![
                        CommandType::Subcommand {
                            name: "kill".to_owned(),
                            help_text: "Force kill the service process".to_owned(),
                            hide: false,
                            children: vec![],
                        },
                        CommandType::Subcommand {
                            name: "info".to_owned(),
                            help_text: "Force kill the service process".to_owned(),
                            hide: false,
                            children: vec![],
                        },
                    ],
                },
                action: None,
            },
        }
    }
}

#[async_trait]
impl CommandProvider for ProcessCliProvider {
    fn get_commands(&self) -> Vec<&CommandConfig> {
        vec![&self.command]
    }

    async fn handle_input(
        mut self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        let Some(matched_command) = matched_command else {
            return Ok(CommandOutput::unhandled())
        };
        if matched_command.matches_subcommand("process") {
            if let Some((name, _)) = matched_command.matches.subcommand() {
                return Ok(match name {
                    "kill" => match self.pid {
                        Some(pid) => {
                            ProcessManager::kill(pid);
                            CommandOutput::handled("Process killed".to_owned())
                        }
                        None => CommandOutput::handled(
                            "Cannot kill process because it is not running".to_owned(),
                        ),
                    },
                    "info" => match self.pid {
                        Some(pid) => CommandOutput::handled(format!(
                            "{:?}",
                            ProcessManager::new(pid).process_info()
                        )),
                        None => CommandOutput::handled(
                            "Cannot fetch process info because it is not running".to_owned(),
                        ),
                    },
                    _ => CommandOutput::unhandled(),
                });
            }
        }
        return Ok(CommandOutput::unhandled());
    }
}
