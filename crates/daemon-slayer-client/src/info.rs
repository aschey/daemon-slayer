use crate::State;
use daemon_slayer_core::Label;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Info {
    pub label: Label,
    pub state: State,
    pub autostart: Option<bool>,
    pub pid: Option<u32>,
    pub last_exit_code: Option<i32>,
    pub id: Option<String>,
}

impl Info {
    #[cfg(feature = "cli")]
    pub fn pretty_print(&self) -> String {
        let mut printer = daemon_slayer_core::cli::Printer::default()
            .with_line("State", self.state.pretty_print())
            .with_line("Autostart", self.pretty_print_autostart())
            .with_line(
                "PID",
                self.pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            );
        if let Some(id) = &self.id {
            printer = printer.with_line("ID", id);
        }
        printer
            .with_line("Exit Code", self.pretty_print_exit_code())
            .with_line("Label", self.label.qualified_name())
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
