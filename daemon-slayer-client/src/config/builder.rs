use super::SystemdConfig;
use super::WindowsConfig;
use crate::platform::ServiceManager;
use crate::Level;
use crate::Manager;
use crate::Result;
use arc_swap::access::DynAccess;
use arc_swap::ArcSwap;
#[cfg(feature = "config")]
use daemon_slayer_core::config::Configurable;
use std::env::consts::EXE_EXTENSION;
use std::env::current_exe;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "config")]
#[derive(Debug, Clone, confique::Config, Default, serde::Deserialize, PartialEq, Eq)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

#[cfg(feature = "config")]
#[derive(Debug, Clone, Default, confique::Config)]
pub struct UserConfig {
    pub(crate) env_vars: Option<Vec<EnvVar>>,
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
    env_vars: Vec<(String, String)>,
    pub(crate) autostart: bool,
    #[cfg_attr(not(platform = "linux"), allow(unused))]
    pub(crate) systemd_config: SystemdConfig,
    #[cfg_attr(not(windows), allow(unused))]
    pub(crate) windows_config: WindowsConfig,
    #[cfg(feature = "config")]
    pub(crate) user_config: Option<Arc<Box<dyn DynAccess<UserConfig> + Send + Sync>>>,
    #[cfg(feature = "config")]
    pub(crate) config_snapshot: UserConfig,
}

#[cfg(feature = "config")]
impl Configurable for Builder {
    type UserConfig = UserConfig;

    fn with_user_config(
        mut self,
        config: Box<dyn DynAccess<Self::UserConfig> + Send + Sync>,
    ) -> Self {
        let c = config.load();
        self.config_snapshot = c.clone();
        self.user_config = Some(Arc::new(config));

        self
    }
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
            env_vars: vec![],
            systemd_config: SystemdConfig::default(),
            windows_config: WindowsConfig::default(),
            #[cfg(feature = "config")]
            user_config: Default::default(),
            #[cfg(feature = "config")]
            config_snapshot: Default::default(),
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
        self.env_vars.push((key.into(), value.into()));
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
        let mut vars = self.env_vars.clone();
        #[cfg(feature = "config")]
        if let Some(config) = &self.user_config {
            let c = config.load();
            if let Some(v) = &c.env_vars {
                for EnvVar { name, value } in v {
                    vars.push((name.to_owned(), value.to_owned()));
                }
            }
        }

        vars
    }

    #[cfg(unix)]
    pub(crate) fn full_args_iter(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.program).chain(self.args_iter())
    }
}
