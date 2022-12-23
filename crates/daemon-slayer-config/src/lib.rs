use std::{
    fs::{create_dir_all, File},
    io::{self, Write},
    marker::PhantomData,
    path::PathBuf,
    sync::Arc,
};

use arc_swap::ArcSwap;
use confique::{json5, toml, yaml, Config};
use daemon_slayer_core::{
    config::{Accessor, CachedConfig, Mergeable},
    Label,
};
use directories::ProjectDirs;
use tracing::debug;
use tracing::error;
#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "server")]
pub mod server;

pub trait Configurable: Config + Default + Send + Sync + Clone + 'static {}

impl<T> Configurable for T where T: Config + Default + Send + Sync + Clone + 'static {}

#[derive(Clone)]
pub enum ConfigFileType {
    Toml,
    Yaml,
    Json5,
}

impl ConfigFileType {
    fn to_extension(&self) -> &str {
        match &self {
            ConfigFileType::Toml => ".toml",
            ConfigFileType::Yaml => ".yaml",
            ConfigFileType::Json5 => ".json5",
        }
    }

    fn to_format_language(&self) -> &str {
        match &self {
            ConfigFileType::Toml => "toml",
            ConfigFileType::Yaml => "yaml",
            ConfigFileType::Json5 => "javascript",
        }
    }
}

#[derive(Clone)]
pub struct AppConfig<T: Configurable> {
    config_file_type: ConfigFileType,
    phantom: PhantomData<T>,
    config_dir: PathBuf,
    filename: String,
    config: Arc<ArcSwap<T>>,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigInitializationError {
    #[error("The user's home directory could not be located")]
    NoHomeDir,
}

#[cfg(feature = "pretty-print")]
#[derive(thiserror::Error, Debug)]
pub enum PrettyPrintError {
    #[error("Error opening {0:#?} for pretty printing: {1:?}")]
    IOFailure(PathBuf, io::Error),
    #[error("Error opening {0:#?} for pretty printing: Syntax parsing error: {1}")]
    SyntaxParsingFailure(PathBuf, String),
    #[error("Error opening {0:#?} for pretty printing: {1}")]
    Other(PathBuf, String),
}

#[cfg(feature = "pretty-print")]
impl PrettyPrintError {
    fn from_bat_error(path: PathBuf, error: bat::error::Error) -> Self {
        match error {
            bat::error::Error::Io(e) => Self::IOFailure(path, e),
            bat::error::Error::UndetectedSyntax(e) => Self::SyntaxParsingFailure(path, e),
            bat::error::Error::UnknownSyntax(e) => Self::SyntaxParsingFailure(path, e),
            e => Self::Other(path, e.to_string()),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigEditError {
    #[error("Error editing config file: {0}")]
    LoadFailure(ConfigLoadError),
    #[error("Error editing config file {0}: {1}")]
    IOFailure(PathBuf, io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Error loding config file {0:#?}: {1}")]
pub struct ConfigLoadError(PathBuf, String);

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

        Ok(Self::from_custom_path(config_file_type, config_dir))
    }

    pub fn from_custom_path(
        config_file_type: ConfigFileType,
        config_dir: impl Into<PathBuf>,
    ) -> Self {
        let config = Arc::new(ArcSwap::new(Arc::new(T::default())));

        let filename = format!("config{}", config_file_type.to_extension());
        Self {
            config_file_type,
            config_dir: config_dir.into(),
            filename,
            phantom: Default::default(),
            config,
        }
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

    pub fn full_path(&self) -> PathBuf {
        self.config_dir.join(&self.filename)
    }

    pub fn edit(&self) -> Result<(), ConfigEditError> {
        let full_path = self.full_path();
        edit::edit_file(&full_path).map_err(|e| ConfigEditError::IOFailure(full_path, e))?;
        self.read_config().map_err(ConfigEditError::LoadFailure)?;
        Ok(())
    }

    #[cfg(feature = "pretty-print")]
    pub fn pretty_print(&self) -> Result<(), PrettyPrintError> {
        let full_path = self.full_path();
        bat::PrettyPrinter::new()
            .input_file(&full_path)
            .grid(true)
            .header(true)
            .paging_mode(bat::PagingMode::QuitIfOneScreen)
            .line_numbers(true)
            .language(self.config_file_type.to_format_language())
            .print()
            .map_err(|e| PrettyPrintError::from_bat_error(full_path, e))?;
        Ok(())
    }

    pub fn contents(&self) -> Result<String, io::Error> {
        let full_path = self.full_path();
        std::fs::read_to_string(&full_path)
            .map_err(|e| io_error(&format!("Error reading config file {full_path:#?}"), e))
    }

    pub fn ensure_config_file(&self) -> Result<(), io::Error> {
        let full_path = self.full_path();
        if full_path.exists() {
            debug!("Not creating config file {full_path:#?} because it already exists");
            return Ok(());
        }

        self.overwrite_config_file()
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

fn io_error(msg: &str, inner: io::Error) -> io::Error {
    io::Error::new(inner.kind(), format!("{msg}: {}", inner))
}
