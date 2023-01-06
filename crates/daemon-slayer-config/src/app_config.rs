use crate::{
    io_error, ConfigEditError, ConfigFileType, ConfigInitializationError, ConfigLoadError,
};
use arc_swap::ArcSwap;
use confique::{json5, toml, yaml, Config};
use daemon_slayer_core::{
    config::{Accessor, CachedConfig, Mergeable},
    Label,
};
use directories::ProjectDirs;
use std::{
    fs::{create_dir_all, File},
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};
use tracing::{debug, error};

pub trait Configurable: Config + Default + Send + Sync + Clone + 'static {}

impl<T> Configurable for T where T: Config + Default + Send + Sync + Clone + 'static {}

#[derive(Clone)]
pub struct AppConfig<T: Configurable> {
    config_file_type: ConfigFileType,
    config_dir: PathBuf,
    filename: String,
    config: Arc<ArcSwap<T>>,
}

impl<T: Configurable> AppConfig<T> {
    pub fn from_config_dir(
        identifier: Label,
        config_file_type: ConfigFileType,
    ) -> Result<Self, ConfigInitializationError> {
        let dirs = ProjectDirs::from(
            &identifier.qualifier,
            &identifier.organization,
            &identifier.application,
        )
        .ok_or(ConfigInitializationError::NoHomeDir)?;

        let config_dir = dirs.config_dir();

        Self::from_custom_path(config_file_type, config_dir)
            .map_err(|e| ConfigInitializationError::CreationFailure(config_dir.to_owned(), e))
    }

    pub fn from_custom_path(
        config_file_type: ConfigFileType,
        config_dir: impl Into<PathBuf>,
    ) -> Result<Self, io::Error> {
        let config = Arc::new(ArcSwap::new(Arc::new(T::default())));

        let filename = format!("config{}", config_file_type.to_extension());
        let instance = Self {
            config_file_type,
            config_dir: config_dir.into(),
            filename,
            config,
        };
        instance.ensure_created()?;
        Ok(instance)
    }

    pub fn ensure_created(&self) -> Result<(), io::Error> {
        let full_path = self.full_path();
        if full_path.exists() {
            debug!("Not creating config file {full_path:#?} because it already exists");
            return Ok(());
        }

        self.overwrite_config_file()
    }

    pub fn with_config_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = filename.into();
        self
    }

    pub fn config_template(&self) -> String {
        match self.config_file_type {
            ConfigFileType::Yaml => yaml::template::<T>(yaml::FormatOptions::default()),
            ConfigFileType::Toml => toml::template::<T>(toml::FormatOptions::default()),
            ConfigFileType::Json5 => json5::template::<T>(json5::FormatOptions::default()),
        }
    }

    pub fn file_type(&self) -> &ConfigFileType {
        &self.config_file_type
    }

    pub fn full_path(&self) -> PathBuf {
        self.config_dir.join(&self.filename)
    }

    pub fn edit(&self) -> Result<(), ConfigEditError> {
        let full_path = self.full_path();
        edit::edit_file(&full_path).map_err(|e| ConfigEditError::IOFailure(full_path, e))?;
        self.read_config().map_err(ConfigEditError::LoadFailure)?;
        Ok(())
    }

    pub fn contents(&self) -> Result<String, io::Error> {
        let full_path = self.full_path();
        std::fs::read_to_string(&full_path)
            .map_err(|e| io_error(&format!("Error reading config file {full_path:#?}"), e))
    }

    pub fn overwrite_config_file(&self) -> Result<(), io::Error> {
        create_dir_all(&self.config_dir).map_err(|e| {
            io_error(
                &format!("Error creating config dir {:#?}", self.config_dir),
                e,
            )
        })?;
        let full_path = self.full_path();
        let mut file = File::create(&full_path)
            .map_err(|e| io_error(&format!("Error creating config file {:#?}", full_path), e))?;

        file.write_all(self.config_template().as_bytes())
            .map_err(|e| io_error("Error writing config template", e))?;
        Ok(())
    }

    pub fn snapshot(&self) -> Arc<T> {
        self.config.load_full()
    }

    pub fn read_config(&self) -> Result<Arc<ArcSwap<T>>, ConfigLoadError> {
        let full_path = self.full_path();
        let val = T::builder()
            .env()
            .file(self.full_path())
            .load()
            .map_err(|e| ConfigLoadError(full_path, e.to_string()))?;
        self.config.store(Arc::new(val));
        Ok(self.config.clone())
    }
}

impl<T, E> Accessor<E> for AppConfig<T>
where
    T: AsRef<E> + Configurable,
    E: Mergeable + Clone + Default + 'static,
{
    fn access(&self) -> CachedConfig<E> {
        match self.read_config() {
            Ok(config) => config.access(),
            Err(e) => {
                error!("Error loading config: {e}");
                self.config.access()
            }
        }
    }
}

#[cfg(test)]
#[path = "./app_config_test.rs"]
mod app_config_test;