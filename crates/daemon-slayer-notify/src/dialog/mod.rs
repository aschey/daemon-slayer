use std::{io, marker::PhantomData};

use super::ShowNotification;
use daemon_slayer_core::async_trait;
use native_dialog::MessageType;
use tap::TapFallible;
use tracing::error;

#[cfg(feature = "cli")]
pub mod cli;

pub trait DialogType {}

#[derive(Clone)]
pub struct Alert;
impl DialogType for Alert {}

#[derive(Clone)]
pub struct Confirm;
impl DialogType for Confirm {}

#[derive(Clone)]
pub struct MessageDialog<T: DialogType> {
    title: String,
    text: String,
    message_type: MessageType,
    _phantom: PhantomData<T>,
}

impl<T: DialogType> Default for MessageDialog<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DialogType> MessageDialog<T> {
    pub fn new() -> Self {
        Self {
            title: Default::default(),
            text: Default::default(),
            message_type: MessageType::Info,
            _phantom: Default::default(),
        }
    }
}

impl<T: DialogType> MessageDialog<T> {
    pub fn with_title(self, title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..self
        }
    }

    pub fn with_text(self, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..self
        }
    }

    pub fn with_type(self, message_type: MessageType) -> Self {
        Self {
            message_type,
            ..self
        }
    }
}

#[async_trait]
impl ShowNotification for MessageDialog<Alert> {
    type Output = ();

    async fn show(&self) -> io::Result<Self::Output> {
        let this = self.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::task::spawn_blocking(move || {
            native_dialog::MessageDialog::default()
                .set_title(&this.title)
                .set_text(&this.text)
                .set_type(this.message_type)
                .show_alert()
                .tap_err(|e| error!("Error showing alert: {e:?}"))
                .ok();

            tx.send(())
                .tap_err(|e| {
                    error!("Error sending dialog signal: {e:?}");
                })
                .ok();
        });
        rx.await
            .tap_err(|e| {
                error!("Error receiving dialog signal {e:?}");
            })
            .ok();
        Ok(())
    }
}

#[async_trait]
impl ShowNotification for MessageDialog<Confirm> {
    type Output = bool;

    async fn show(&self) -> io::Result<Self::Output> {
        let this = self.clone();

        tokio::task::spawn_blocking(move || {
            native_dialog::MessageDialog::default()
                .set_title(&this.title)
                .set_text(&this.text)
                .set_type(this.message_type)
                .show_confirm()
                .map_err(|e| match e {
                    native_dialog::Error::IoFailure(e) => e,
                    e => io::Error::new(io::ErrorKind::Other, e.to_string()),
                })
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
    }
}
