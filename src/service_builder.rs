use std::env::current_exe;

use crate::{
    platform,
    service_manager::{Result, ServiceManager},
};

#[derive(Debug, PartialEq, Eq)]
pub enum ServiceLevel {
    System,
    User,
}

pub struct ServiceBuilder {
    pub(crate) name: String,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) display_name: String,
    #[cfg_attr(unix, allow(unused))]
    pub(crate) description: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) service_level: ServiceLevel,
}

impl ServiceBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            display_name: name,
            description: "".to_owned(),
            args: vec![],
            program: current_exe().unwrap().to_string_lossy().to_string(),
            service_level: ServiceLevel::System,
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

    pub fn with_service_level(self, service_level: ServiceLevel) -> Self {
        Self {
            service_level,
            ..self
        }
    }

    pub fn build(self) -> Result<platform::Manager> {
        platform::Manager::from_builder(self)
    }

    pub(crate) fn args_iter(&self) -> impl Iterator<Item = &String> {
        self.args.iter()
    }

    #[cfg(unix)]
    pub(crate) fn full_args_iter(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.program).chain(self.args_iter())
    }
}
