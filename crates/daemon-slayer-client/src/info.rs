use crate::State;
use daemon_slayer_core::cli::Printer;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Info {
    pub state: State,
    pub autostart: Option<bool>,
    pub pid: Option<u32>,
    pub last_exit_code: Option<i32>,
}

impl Info {
    pub fn pretty_print(&self) -> String {
        Printer::default()
            .with_line("State", self.state.pretty_print())
            .with_line("Autostart", self.pretty_print_autostart())
            .with_line(
                "PID",
                self.pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            )
            .with_line("Exit Code", self.pretty_print_exit_code())
            .print()
    }

    fn pretty_print_autostart(&self) -> String {
        match self.autostart {
            Some(true) => "Enabled".blue().to_string(),
            Some(false) => "Disabled".yellow().to_string(),
            None => "N/A".to_string(),
        }
    }

    fn pretty_print_exit_code(&self) -> String {
        match self.last_exit_code {
            Some(0) => "0".green().to_string(),
            Some(val) => val.to_string().yellow().to_string(),
            None => "N/A".to_string(),
        }
    }
}
