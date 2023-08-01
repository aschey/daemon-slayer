mod builder;
pub use builder::*;
mod level;
pub mod systemd;
pub mod windows;
pub use level::*;
mod environment_variable;
pub use environment_variable::*;
mod user_config;
pub use user_config::*;

use self::{systemd::SystemdConfig, windows::WindowsConfig};
use daemon_slayer_core::{config::CachedConfig, Label};

#[derive(Clone, Debug)]
pub struct Config {
    pub label: Label,
    pub display_name: Option<String>,
    pub description: String,
    pub program: Program,
    pub arguments: Vec<String>,
    pub service_level: Level,
    pub autostart: bool,
    pub systemd_config: SystemdConfig,
    pub windows_config: WindowsConfig,
    pub user_config: CachedConfig<UserConfig>,
    pub service_type: ServiceType,
}

impl From<Builder> for Config {
    fn from(value: Builder) -> Self {
        Self {
            label: value.label,
            display_name: value.display_name,
            description: value.description,
            program: value.program,
            arguments: value.arguments,
            service_level: value.service_level,
            autostart: value.autostart,
            systemd_config: value.systemd_config,
            windows_config: value.windows_config,
            user_config: value.user_config,
            service_type: value.service_type,
        }
    }
}

impl Config {
    #[cfg(feature = "cli")]
    pub fn pretty_print(&self) -> String {
        use owo_colors::OwoColorize;

        let printer = daemon_slayer_core::cli::Printer::default()
            .with_line("Label".cyan().to_string(), self.label.qualified_name())
            .with_line(
                "Display Name".cyan().to_string(),
                if let Some(display_name) = &self.display_name {
                    display_name.to_owned()
                } else {
                    "N/A".dimmed().to_string()
                },
            )
            .with_line("Description".cyan().to_string(), &self.description)
            .with_line("Program".cyan().to_string(), self.program.full_name())
            .with_line("Arguments".cyan().to_string(), self.arguments.join(" "))
            .with_line("Level".cyan().to_string(), self.service_level.to_string())
            .with_line(
                "Autostart".cyan().to_string(),
                if self.autostart {
                    "Enabled"
                } else {
                    "Disabled"
                },
            )
            .extend_from(self.user_config.load().pretty_printer());

        #[cfg(windows)]
        let printer = printer.extend_from(self.windows_config.pretty_printer());
        #[cfg(target_os = "linux")]
        let printer = printer.extend_from(self.systemd_config.pretty_printer());

        printer.print()
    }
}
