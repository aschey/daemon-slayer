use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::{mpsc, Arc},
};

pub use color_eyre::config::Theme;
use color_eyre::Report;
use daemon_slayer_core::Label;
use once_cell::sync::OnceCell;
use tap::TapFallible;
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
    label: Label,
    #[cfg(feature = "notify")]
    notify: bool,
    #[cfg(feature = "notify")]
    notification_builder: Arc<
        Box<
            dyn Fn(
                    daemon_slayer_core::notify::Notification,
                ) -> daemon_slayer_core::notify::Notification
                + Send
                + Sync,
        >,
    >,
}

impl ErrorHandler {
    pub fn new(label: Label) -> Self {
        Self {
            label,
            theme: Theme::dark(),
            write_to_stdout: false,
            write_to_stderr: true,
            log: false,
            #[cfg(feature = "notify")]
            notify: false,
            #[cfg(feature = "notify")]
            notification_builder: Arc::new(Box::new(|notification| {
                let app = notification.label.application.clone();
                notification.summary(format!("Application {app} encountered a fatal error"))
            })),
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
    pub fn with_notify(self, notify: bool) -> Self {
        Self { notify, ..self }
    }

    #[cfg(feature = "notify")]
    pub fn with_notification_builder<F>(self, builder: F) -> Self
    where
        F: Fn(daemon_slayer_core::notify::Notification) -> daemon_slayer_core::notify::Notification
            + Send
            + Sync
            + 'static,
    {
        Self {
            notification_builder: Arc::new(Box::new(builder)),
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
        if self.notify {
            let notification = (self.notification_builder)(
                daemon_slayer_core::notify::Notification::new(self.label.clone()),
            );
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let (tx, rx) = mpsc::channel();
                handle.spawn(async move {
                    notification
                        .show()
                        .await
                        .tap_err(|e| error!("Failed to show notification: {e:?}"))
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
                            .tap_err(|e| error!("Failed to show notification: {e:?}"))
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
        let handler = HANDLER
            .get()
            .cloned()
            .unwrap_or_else(|| ErrorHandler::new(Label::default()));

        handler.write_output(format!("{:?}", self.report));
        handler.show_notification();
        Ok(())
    }
}
