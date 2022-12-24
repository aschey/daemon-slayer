use std::{io, path::PathBuf};

use crate::{AppConfig, ConfigFileType, Configurable};

#[derive(thiserror::Error, Debug)]
pub enum PrettyPrintError {
    #[error("Error opening {0:#?} for pretty printing: {1:?}")]
    IOFailure(PathBuf, io::Error),
    #[error("Error opening {0:#?} for pretty printing: Syntax parsing error: {1}")]
    SyntaxParsingFailure(PathBuf, String),
    #[error("Error opening {0:#?} for pretty printing: {1}")]
    Other(PathBuf, String),
}

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

pub struct PrettyPrintOptions {
    pub color: bool,
}

impl Default for PrettyPrintOptions {
    fn default() -> Self {
        Self { color: true }
    }
}

impl ConfigFileType {
    fn to_format_language(&self) -> &str {
        match &self {
            ConfigFileType::Toml => "toml",
            ConfigFileType::Yaml => "yaml",
            ConfigFileType::Json5 => "javascript",
        }
    }
}

impl<T: Configurable> AppConfig<T> {
    pub fn pretty_print(&self, options: PrettyPrintOptions) -> Result<(), PrettyPrintError> {
        let full_path = self.full_path();
        bat::PrettyPrinter::new()
            .input_file(&full_path)
            .grid(true)
            .header(true)
            .paging_mode(bat::PagingMode::QuitIfOneScreen)
            .line_numbers(true)
            .language(self.config_file_type.to_format_language())
            .colored_output(options.color)
            .print()
            .map_err(|e| PrettyPrintError::from_bat_error(full_path, e))?;
        Ok(())
    }
}
