use std::env::current_exe;

#[derive(Debug, PartialEq, Eq)]
pub enum ServiceLevel {
    System,
    User,
}

pub struct ServiceConfig {
    pub(crate) name: String,
    pub(crate) display_name: String,
    pub(crate) description: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) service_level: ServiceLevel,
}

impl ServiceConfig {
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

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    pub fn with_args<T: Into<String>>(mut self, args: impl IntoIterator<Item = T>) -> Self {
        self.args = args.into_iter().map(|a| a.into()).collect();
        self
    }

    pub fn with_service_level(mut self, service_level: ServiceLevel) -> Self {
        self.service_level = service_level;
        self
    }

    pub(crate) fn args_iter(&self) -> impl Iterator<Item = &String> {
        self.args.iter()
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn full_args_iter(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.program).chain(self.args_iter())
    }
}
