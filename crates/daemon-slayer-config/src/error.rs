use std::{io, path::PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum ConfigInitializationError {
    #[error("The user's home directory could not be located")]
    NoHomeDir,
    #[error("Error creating config file {0}: {1}")]
    CreationFailure(PathBuf, io::Error),
    #[error("{0}")]
    ConfigLoadError(#[from] ConfigLoadError),
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigEditError {
    #[error("Error editing config file: {0}")]
    LoadFailure(ConfigLoadError),
    #[error("Error editing config file {0}: {1}")]
    IOFailure(PathBuf, io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Error loading config file {0:#?}: {1}")]
pub struct ConfigLoadError(pub(crate) PathBuf, pub(crate) String);

pub(crate) fn io_error(msg: &str, inner: io::Error) -> io::Error {
    io::Error::new(inner.kind(), format!("{msg}: {inner}"))
}
