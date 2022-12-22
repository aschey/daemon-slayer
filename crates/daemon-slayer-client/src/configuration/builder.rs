use super::systemd::SystemdConfiguration;
use super::windows::WindowsConfiguration;
use super::EnvironmentVariable;
use super::Level;
use super::UserConfiguration;
use crate::get_manager;
use crate::Manager;
use daemon_slayer_core::config::Accessor;
use daemon_slayer_core::config::CachedConfig;
use daemon_slayer_core::Label;
use std::env::consts::EXE_EXTENSION;
use std::env::current_exe;
use std::io;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Builder {
    pub(crate) label: Label,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) display_name: Option<String>,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) description: String,
    pub(crate) program: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) service_level: Level,
    pub(crate) autostart: bool,
    #[cfg_attr(not(platform = "linux"), allow(unused))]
    pub(crate) systemd_configuration: SystemdConfiguration,
    #[cfg_attr(not(windows), allow(unused))]
    pub(crate) windows_configuration: WindowsConfiguration,
    pub(crate) user_configuration: CachedConfig<UserConfiguration>,
}

impl Builder {
    pub fn new(label: Label) -> Self {
        Self {
            label,
            display_name: None,
            description: "".to_owned(),
            arguments: vec![],
            program: current_exe()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            service_level: Level::System,
            autostart: false,
            systemd_configuration: Default::default(),
            windows_configuration: Default::default(),
            user_configuration: Default::default(),
        }
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_program(mut self, program: impl Into<PathBuf>) -> Self {
        let mut program = program.into();
        program.set_extension(EXE_EXTENSION);
        self.program = program.to_string_lossy().to_string();
        self
    }

    pub fn with_args<T: Into<String>>(mut self, args: impl IntoIterator<Item = T>) -> Self {
        self.arguments = args.into_iter().map(|a| a.into()).collect();
        self
    }

    pub fn with_service_level(mut self, service_level: Level) -> Self {
        self.service_level = service_level;
        self
    }

    pub fn with_autostart(mut self, autostart: bool) -> Self {
        self.autostart = autostart;
        self
    }

    pub fn with_environment_variable(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.user_configuration
            .edit()
            .environment_variables
            .push(EnvironmentVariable {
                name: key.into(),
                value: value.into(),
            });
        self
    }

    pub fn with_systemd_configuration(
        mut self,
        systemd_configuration: SystemdConfiguration,
    ) -> Self {
        self.systemd_configuration = systemd_configuration;
        self
    }

    pub fn with_windows_configuration(
        mut self,
        windows_configuration: WindowsConfiguration,
    ) -> Self {
        self.windows_configuration = windows_configuration;
        self
    }

    pub fn with_user_configuration(
        mut self,
        config: impl Accessor<UserConfiguration> + Send + Sync + 'static,
    ) -> Self {
        self.user_configuration = config.access();
        self
    }

    pub fn build(self) -> Result<Box<dyn Manager>, io::Error> {
        get_manager(self)
    }

    pub(crate) fn arguments_iter(&self) -> impl Iterator<Item = &String> {
        self.arguments.iter()
    }

    #[cfg_attr(target_os = "macos", allow(unused))]
    pub(crate) fn is_user(&self) -> bool {
        self.service_level == Level::User
    }

    pub(crate) fn display_name(&self) -> &str {
        self.display_name
            .as_deref()
            .unwrap_or(self.label.application.as_str())
    }

    pub(crate) fn environment_variables(&self) -> Vec<(String, String)> {
        self.user_configuration
            .load()
            .environment_variables
            .iter()
            .map(|pair| (pair.name.to_owned(), pair.value.to_owned()))
            .collect()
    }

    #[cfg(unix)]
    pub(crate) fn full_arguments_iter(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.program).chain(self.arguments_iter())
    }
}
