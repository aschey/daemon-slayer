pub use color_eyre::config::Theme;
use daemon_slayer_core::BoxedError;
use tracing::error;
#[cfg(feature = "cli")]
pub mod cli;

pub use color_eyre;

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

    pub fn install(self) -> Result<(), BoxedError> {
        let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
            .add_default_filters()
            .theme(self.theme)
            .into_hooks();

        eyre_hook.install()?;

        std::panic::set_hook(Box::new(move |pi| {
            if self.log {
                error!("{}", panic_hook.panic_report(pi));
            }
            if self.write_to_stdout {
                println!("{}", panic_hook.panic_report(pi));
            }
            if self.write_to_stderr {
                eprintln!("{}", panic_hook.panic_report(pi));
            }
        }));
        Ok(())
    }
}
