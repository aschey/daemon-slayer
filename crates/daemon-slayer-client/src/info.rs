use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::State;

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Status {
    pub state: State,
    pub autostart: Option<bool>,
    pub pid: Option<u32>,
    pub last_exit_code: Option<i32>,
    pub id: Option<String>,
}

impl Status {
    #[cfg(feature = "cli")]
    pub fn pretty_print(&self) -> String {
        let mut printer = daemon_slayer_core::cli::Printer::default()
            .with_line("State", self.state.pretty_print())
            .with_optional_line("Autostart", self.pretty_print_autostart())
            .with_optional_line("PID", self.pid.map(|p| p.to_string()));
        if let Some(id) = &self.id {
            printer = printer.with_line("ID", id);
        }
        printer
            .with_optional_line("Exit Code", self.pretty_print_exit_code())
            .print()
    }

    fn pretty_print_autostart(&self) -> Option<String> {
        match self.autostart {
            Some(true) => Some("Enabled".blue().to_string()),
            Some(false) => Some("Disabled".yellow().to_string()),
            None => None,
        }
    }

    fn pretty_print_exit_code(&self) -> Option<String> {
        match self.last_exit_code {
            Some(0) => Some("0".green().to_string()),
            Some(val) => Some(val.to_string().yellow().to_string()),
            None => None,
        }
    }
}
