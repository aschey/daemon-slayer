#[derive(Clone, Debug)]
pub enum ConfigFileType {
    Toml,
    Yaml,
    Json5,
}

impl ConfigFileType {
    pub fn to_extension(&self) -> &str {
        match &self {
            ConfigFileType::Toml => ".toml",
            ConfigFileType::Yaml => ".yaml",
            ConfigFileType::Json5 => ".json5",
        }
    }
}
