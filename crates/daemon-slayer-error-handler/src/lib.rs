use std::{
    error::Error,
    fmt::{Debug, Display},
};

pub use color_eyre::config::Theme;
use color_eyre::Report;
use once_cell::sync::OnceCell;
use tracing::error;
#[cfg(feature = "cli")]
pub mod cli;

pub use color_eyre;

static HANDLER: OnceCell<ErrorHandler> = OnceCell::new();

#[derive(thiserror::Error, Debug)]
#[error("Unable to install error handler: {0}")]
pub struct HookInstallError(String);

#[derive(Clone)]
pub struct ErrorHandler {
    theme: Theme,
    write_to_stdout: bool,
    write_to_stderr: bool,
    log: bool,
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self {
            theme: Theme::dark(),
            write_to_stdout: false,
            write_to_stderr: true,
            log: false,
        }
    }
}

impl ErrorHandler {
    pub fn with_theme(self, theme: Theme) -> Self {
        Self { theme, ..self }
    }

    pub fn with_write_to_stdout(self, write_to_stdout: bool) -> Self {
        Self {
            write_to_stdout,
            ..self
        }
    }

    pub fn with_write_to_stderr(self, write_to_stderr: bool) -> Self {
        Self {
            write_to_stderr,
            ..self
        }
    }

    pub fn with_log(self, log: bool) -> Self {
        Self { log, ..self }
    }

    pub fn install(self) -> Result<(), HookInstallError> {
        let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
            .add_default_filters()
            .theme(self.theme)
            .into_hooks();

        HANDLER
            .set(self.clone())
            .map_err(|_| HookInstallError("Handler was already set".to_owned()))?;

        eyre_hook
            .install()
            .map_err(|e| HookInstallError(e.to_string()))?;

        std::panic::set_hook(Box::new(move |pi| {
            self.write_output(panic_hook.panic_report(pi).to_string());
        }));
        Ok(())
    }

    fn write_output(&self, output: impl Display) {
        if self.log {
            error!("{output}");
        }
        if self.write_to_stdout {
            println!("{output}");
        }
        if self.write_to_stderr {
            eprintln!("{output}");
        }
    }
}

pub struct ErrorSink {
    report: Report,
}

impl ErrorSink {
    pub fn new(source: impl Into<color_eyre::Report>) -> Self {
        Self {
            report: source.into(),
        }
    }

    pub fn from_error(source: Box<dyn Error + Send + Sync + 'static>) -> Self {
        Self::new(color_eyre::eyre::eyre!(source))
    }
}

impl<R> From<R> for ErrorSink
where
    R: Into<Report>,
{
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl Debug for ErrorSink {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let handler = HANDLER.get().cloned().unwrap_or_default();
        handler.write_output(format!("{:?}", self.report));
        Ok(())
    }
}
