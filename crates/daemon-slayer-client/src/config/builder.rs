use super::SystemdConfig;
use super::WindowsConfig;
use crate::platform::ServiceManager;
use crate::Level;
use crate::Manager;
use crate::Result;
use arc_swap::access::{DynAccess, Map};
use arc_swap::ArcSwap;
use daemon_slayer_core::config::Accessor;
use daemon_slayer_core::config::CachedConfig;
use daemon_slayer_core::config::Mergeable;
use std::env::consts::EXE_EXTENSION;
use std::env::current_exe;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "config", derive(confique::Config, serde::Deserialize))]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "config", derive(confique::Config))]
pub struct UserConfig {
    #[cfg_attr(feature="config",config(default=[]))]
    pub(crate) env_vars: Vec<EnvVar>,
}

impl Mergeable for UserConfig {
    fn merge(user_config: Option<&Self>, app_config: &Self) -> Self {
        let mut vars = vec![];
        if let Some(user_config) = user_config {
            vars.extend_from_slice(&user_config.env_vars);
        }

        vars.extend_from_slice(&app_config.env_vars);
        UserConfig { env_vars: vars }
    }
}

#[derive(Clone)]
pub struct Builder {
    pub(crate) name: String,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) display_name: String,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) description: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) service_level: Level,
    pub(crate) autostart: bool,
    #[cfg_attr(not(platform = "linux"), allow(unused))]
    pub(crate) systemd_config: SystemdConfig,
    #[cfg_attr(not(windows), allow(unused))]
    pub(crate) windows_config: WindowsConfig,
    pub(crate) user_config: CachedConfig<UserConfig>,
}

impl Builder {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            display_name: name,
            description: "".to_owned(),
            args: vec![],
            program: current_exe().unwrap().to_string_lossy().to_string(),
            service_level: Level::System,
            autostart: false,
            systemd_config: SystemdConfig::default(),
            windows_config: WindowsConfig::default(),
            user_config: Default::default(),
        }
    }

    pub fn with_display_name(self, display_name: impl Into<String>) -> Self {
        Self {
            display_name: display_name.into(),
            ..self
        }
    }

    pub fn with_description(self, description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            ..self
        }
    }

    pub fn with_program(self, program: impl Into<PathBuf>) -> Self {
        let mut program = program.into();
        program.set_extension(EXE_EXTENSION);
        Self {
            program: program.to_string_lossy().to_string(),
            ..self
        }
    }

    pub fn with_args<T: Into<String>>(self, args: impl IntoIterator<Item = T>) -> Self {
        Self {
            args: args.into_iter().map(|a| a.into()).collect(),
            ..self
        }
    }

    pub fn with_service_level(self, service_level: Level) -> Self {
        Self {
            service_level,
            ..self
        }
    }

    pub fn with_autostart(self, autostart: bool) -> Self {
        Self { autostart, ..self }
    }

    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.user_config.edit().env_vars.push(EnvVar {
            name: key.into(),
            value: value.into(),
        });
        self
    }

    pub fn with_systemd_config(self, systemd_config: SystemdConfig) -> Self {
        Self {
            systemd_config,
            ..self
        }
    }

    pub fn with_windows_config(self, windows_config: WindowsConfig) -> Self {
        Self {
            windows_config,
            ..self
        }
    }

    pub fn with_user_config(
        mut self,
        config: impl Accessor<UserConfig> + Send + Sync + 'static,
    ) -> Self {
        self.user_config = config.access();
        self
    }

    pub fn build(self) -> Result<ServiceManager> {
        ServiceManager::from_builder(self)
    }

    pub(crate) fn args_iter(&self) -> impl Iterator<Item = &String> {
        self.args.iter()
    }

    pub(crate) fn is_user(&self) -> bool {
        self.service_level == Level::User
    }

    pub(crate) fn env_vars(&self) -> Vec<(String, String)> {
        self.user_config
            .load()
            .env_vars
            .iter()
            .map(|pair| (pair.name.to_owned(), pair.value.to_owned()))
            .collect()
    }

    #[cfg(unix)]
    pub(crate) fn full_args_iter(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.program).chain(self.args_iter())
    }
}
