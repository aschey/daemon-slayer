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
