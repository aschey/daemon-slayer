#[derive(Default, Clone)]
pub struct SystemdConfig {
    pub(crate) after: Vec<String>,
}

impl SystemdConfig {
    pub fn new() -> Self {
        Self { after: vec![] }
    }

    pub fn with_after_target(mut self, after: impl Into<String>) -> Self {
        self.after.push(after.into());
        self
    }
}
