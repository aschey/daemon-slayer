use std::marker::PhantomData;
use std::path::PathBuf;

use daemon_slayer_core::Label;

use crate::{AppConfig, ConfigFileType, ConfigInitializationError, Configurable};

pub enum ConfigDir {
    ProjectDir(Label),
    Custom(PathBuf),
}

pub struct AppConfigBuilder<T: Configurable> {
    pub(crate) config_dir: ConfigDir,
    pub(crate) config_file_type: ConfigFileType,
    pub(crate) config_filename: Option<String>,
    _phantom: PhantomData<T>,
}

impl<T: Configurable> AppConfigBuilder<T> {
    pub(crate) fn new(config_dir: ConfigDir) -> Self {
        let config_file_type = ConfigFileType::Toml;
        Self {
            config_dir,
            config_file_type,
            config_filename: None,
            _phantom: Default::default(),
        }
    }

    pub fn with_config_file_type(mut self, config_file_type: ConfigFileType) -> Self {
        self.config_file_type = config_file_type;
        self
    }

    pub fn with_config_filename(mut self, filename: impl Into<String>) -> Self {
        self.config_filename = Some(filename.into());
        self
    }

    pub fn build(self) -> Result<AppConfig<T>, ConfigInitializationError> {
        AppConfig::<T>::from_builder(self)
    }
}
