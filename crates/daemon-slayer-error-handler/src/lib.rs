#[cfg(feature = "cli")]
pub mod cli;

use std::fmt::{Debug, Display};
use std::sync::OnceLock;

pub use color_eyre;
use color_eyre::Report;
pub use color_eyre::config::Theme;
#[cfg(feature = "notify")]
use tap::TapFallible;
use tracing::error;

static HANDLER: OnceLock<ErrorHandler> = OnceLock::new();

#[derive(thiserror::Error, Debug)]
#[error("Unable to install error handler: {0}")]
pub struct HookInstallError(String);

#[derive(Clone)]
pub struct ErrorHandler {
    theme: Theme,
    pub(crate) write_to_stdout: bool,
    pub(crate) write_to_stderr: bool,
    pub(crate) log: bool,
    #[cfg(feature = "notify")]
    notification: Option<
        std::sync::Arc<
            dyn daemon_slayer_core::notify::AsyncNotification<Output = ()> + Send + Sync + 'static,
        >,
    >,
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self {
            theme: Theme::dark(),
            write_to_stdout: false,
            write_to_stderr: true,
            log: false,
            #[cfg(feature = "notify")]
            notification: None,
        }
    }

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

    #[cfg(feature = "notify")]
    pub fn with_notification<N>(self, notification: N) -> Self
    where
        N: daemon_slayer_core::notify::AsyncNotification<Output = ()> + Send + Sync + 'static,
    {
        Self {
            notification: Some(std::sync::Arc::new(notification)),
            ..self
        }
    }

    #[cfg(feature = "notify")]
    pub(crate) fn with_dyn_notification(
        self,
        notification: std::sync::Arc<
            dyn daemon_slayer_core::notify::AsyncNotification<Output = ()> + Send + Sync + 'static,
        >,
    ) -> Self {
        Self {
            notification: Some(notification),
            ..self
        }
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
            self.show_notification();
        }));
        Ok(())
    }

    fn show_notification(&self) {
        #[cfg(feature = "notify")]
        if let Some(notification) = self.notification.clone() {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let (tx, rx) = std::sync::mpsc::channel();
                handle.spawn(async move {
                    notification
                        .show()
                        .await
                        .tap_err(|e| error!("Error showing notification: {e}"))
                        .ok();
                    tx.send(()).ok();
                });
                rx.recv().ok();
            } else {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        notification
                            .show()
                            .await
                            .tap_err(|e| error!("Error showing notification: {e}"))
                            .ok();
                    });
            }
        }
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
        let handler = HANDLER.get().cloned().unwrap_or_default();
        let report = source.into();
        if handler.log {
            error!("{:?}", report);
        }

        handler.show_notification();
        Self { report }
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
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(&format!("{:?}", self.report))
    }
}
