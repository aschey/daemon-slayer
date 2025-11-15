#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SocketActivationBehavior {
    EnableAll,
    #[default]
    EnableSocket,
}

#[derive(Default, Clone, Debug)]
pub struct SystemdConfig {
    pub(crate) after: Vec<String>,
    pub(crate) socket_activation_behavior: SocketActivationBehavior,
}

impl SystemdConfig {
    pub fn with_after_target(mut self, after: impl Into<String>) -> Self {
        self.after.push(after.into());
        self
    }

    pub fn with_socket_activation_behavior(mut self, behavior: SocketActivationBehavior) -> Self {
        self.socket_activation_behavior = behavior;
        self
    }

    #[cfg(feature = "cli")]
    pub fn pretty_printer(&self) -> daemon_slayer_core::cli::Printer {
        use owo_colors::OwoColorize;

        daemon_slayer_core::cli::Printer::default().with_optional_line(
            "After Targets".cyan().to_string(),
            if self.after.is_empty() {
                None
            } else {
                Some(self.after.join(","))
            },
        )
    }
}
