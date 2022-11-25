pub struct App {
    pub qualifier: String,
    pub organization: String,
    pub application: String,
}

impl App {
    pub fn full_name(&self) -> String {
        format!(
            "{}.{}.{}",
            self.qualifier, self.organization, self.application
        )
    }
}
