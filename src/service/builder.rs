use crate::service::manager::Manager;
use crate::service::Result;
use std::env::current_exe;

use super::platform::ServiceManager;
use super::Level;

pub struct Builder {
    pub(crate) name: String,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) display_name: String,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) description: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) service_level: Level,
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

    pub fn with_program(self, program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
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

    pub fn build(self) -> Result<ServiceManager> {
        ServiceManager::from_builder(self)
    }

    pub(crate) fn args_iter(&self) -> impl Iterator<Item = &String> {
        self.args.iter()
    }

    #[cfg(unix)]
    pub(crate) fn full_args_iter(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.program).chain(self.args_iter())
    }
}
