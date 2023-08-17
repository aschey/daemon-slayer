use std::env::consts::EXE_EXTENSION;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "docker")]
use bollard::container;
use daemon_slayer_core::config::{Accessor, CachedConfig};
use daemon_slayer_core::process::get_admin_var;
use daemon_slayer_core::{CommandArg, Label};
use derivative::Derivative;

use super::systemd::SystemdConfig;
use super::windows::WindowsConfig;
use super::{EnvironmentVariable, Level, UserConfig};
use crate::{get_manager, ServiceManager};

#[derive(thiserror::Error, Debug)]
pub enum IntoProgramError {
    #[error("The program path contains invalid UTF-8")]
    InvalidUtf8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    name: String,
    full_name: String,
}

impl Program {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, IntoProgramError> {
        let name: PathBuf = path.into();

        let full_name = name
            .with_extension(EXE_EXTENSION)
            .to_str()
            .ok_or(IntoProgramError::InvalidUtf8)?
            .to_owned();
        Ok(Program {
            name: name.to_string_lossy().to_string(),
            full_name,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn full_name(&self) -> &str {
        &self.full_name
    }
}

impl TryFrom<PathBuf> for Program {
    type Error = IntoProgramError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Program::new(value)
    }
}

impl TryFrom<String> for Program {
    type Error = IntoProgramError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Program::new(value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ServiceType {
    Native,
    Container,
}

#[cfg(feature = "docker")]
pub type ContainerConfigFn = dyn Fn(&mut container::Config<String>) + Send + Sync;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct Builder {
    pub(crate) label: Label,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) display_name: Option<String>,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) description: String,
    pub(crate) program: Program,
    pub(crate) arguments: Vec<String>,
    pub(crate) service_level: Level,
    pub(crate) autostart: bool,
    #[cfg_attr(not(platform = "linux"), allow(unused))]
    pub(crate) systemd_config: SystemdConfig,
    #[cfg_attr(not(windows), allow(unused))]
    pub(crate) windows_config: WindowsConfig,
    pub(crate) user_config: CachedConfig<UserConfig>,
    pub(crate) service_type: ServiceType,
    #[cfg(feature = "docker")]
    #[derivative(Debug = "ignore")]
    pub(crate) configure_container: Option<Arc<Box<ContainerConfigFn>>>,
}

impl Builder {
    pub fn new(label: Label, program: Program) -> Self {
        Self {
            label,
            display_name: None,
            description: "".to_owned(),
            arguments: vec![],
            program,
            service_level: Level::System,
            autostart: false,
            systemd_config: Default::default(),
            windows_config: Default::default(),
            user_config: Default::default(),
            service_type: ServiceType::Native,
            #[cfg(feature = "docker")]
            configure_container: None,
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

    pub fn with_args<'a>(mut self, args: impl IntoIterator<Item = &'a CommandArg>) -> Self {
        self.arguments
            .extend(args.into_iter().map(|a| a.to_string()));
        self
    }

    pub fn with_arg(mut self, arg: &CommandArg) -> Self {
        self.arguments.push(arg.to_string());
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
        self.user_config
            .edit()
            .environment_variables
            .push(EnvironmentVariable {
                name: key.into(),
                value: value.into(),
            });
        self
    }

    pub fn with_systemd_config(mut self, systemd_config: SystemdConfig) -> Self {
        self.systemd_config = systemd_config;
        self
    }

    pub fn with_windows_config(mut self, windows_config: WindowsConfig) -> Self {
        self.windows_config = windows_config;
        self
    }

    pub fn with_user_config(
        mut self,
        config: impl Accessor<UserConfig> + Send + Sync + 'static,
    ) -> Self {
        let current_config = self.user_config.load();
        self.user_config = config.access();

        // re-add any vars that were already added
        for var in current_config.environment_variables {
            self = self.with_environment_variable(var.name, var.value);
        }
        self
    }

    pub fn with_service_type(mut self, service_type: ServiceType) -> Self {
        self.service_type = service_type;
        self
    }

    #[cfg(feature = "docker")]
    pub fn with_configure_container(
        mut self,
        configure_container: impl Fn(&mut container::Config<String>) + Send + Sync + 'static,
    ) -> Self {
        self.configure_container = Some(Arc::new(Box::new(configure_container)));
        self
    }

    pub async fn build(self) -> io::Result<ServiceManager> {
        get_manager(self).await
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
        let mut vars: Vec<_> = self
            .user_config
            .load()
            .environment_variables
            .iter()
            .map(|pair| (pair.name.to_owned(), pair.value.to_owned()))
            .collect();
        if !self.is_user() {
            vars.push((get_admin_var(&self.label), "1".to_owned()))
        }
        vars
    }

    #[cfg(unix)]
    pub(crate) fn full_arguments_iter(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.program.full_name).chain(self.arguments_iter())
    }
}
