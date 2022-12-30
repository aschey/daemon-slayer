#[derive(Clone)]
pub enum ConfigFileType {
    Toml,
    Yaml,
    Json5,
}

impl ConfigFileType {
    pub(crate) fn to_extension(&self) -> &str {
        match &self {
            ConfigFileType::Toml => ".toml",
            ConfigFileType::Yaml => ".yaml",
            ConfigFileType::Json5 => ".json5",
        }
    }
}
