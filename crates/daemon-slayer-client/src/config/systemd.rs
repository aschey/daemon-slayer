#[derive(Default, Clone, Debug)]
pub struct SystemdConfig {
    pub(crate) after: Vec<String>,
}

impl SystemdConfig {
    pub fn with_after_target(mut self, after: impl Into<String>) -> Self {
        self.after.push(after.into());
        self
    }

    #[cfg(feature = "cli")]
    pub fn pretty_printer(&self) -> daemon_slayer_core::cli::Printer {
        use owo_colors::OwoColorize;

        daemon_slayer_core::cli::Printer::default().with_line(
            "After Targets".cyan().to_string(),
            if self.after.is_empty() {
                "N/A".dimmed().to_string()
            } else {
                self.after.join(",")
            },
        )
    }
}
