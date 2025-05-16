use std::env::{self, current_exe};
use std::io;
use std::marker::PhantomData;

use async_trait::async_trait;
use daemon_slayer_core::notify::BlockingNotification;
use daemon_slayer_core::{Label, process};
use native_dialog::{DialogBuilder, MessageLevel};

use super::AsyncNotification;

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
    label: Label,
    title: String,
    text: String,
    message_level: MessageLevel,
    _phantom: PhantomData<T>,
}

impl<T: DialogType> MessageDialog<T> {
    pub fn new(label: Label) -> Self {
        Self {
            title: Default::default(),
            text: Default::default(),
            label,
            message_level: MessageLevel::Info,
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

    pub fn with_level(self, message_level: MessageLevel) -> Self {
        Self {
            message_level,
            ..self
        }
    }

    fn to_args(&self) -> String {
        let message_type = match self.message_level {
            MessageLevel::Info => "info",
            MessageLevel::Warning => "warning",
            MessageLevel::Error => "error",
        };
        let mut args = format!("\"{}\" -m \"{}\"", self.text, message_type);
        if !self.title.is_empty() {
            args += &format!(" -t \"{}\"", self.title);
        }

        args
    }
}

#[async_trait]
impl AsyncNotification for MessageDialog<Alert> {
    type Output = ();

    async fn show(&self) -> io::Result<Self::Output> {
        let is_admin = matches!(
            env::var(process::get_admin_var(&self.label))
                .map(|v| v.to_lowercase())
                .as_deref(),
            Ok("1" | "true")
        );

        if is_admin || cfg!(target_os = "macos") {
            let cmd = format!(
                "{} alert {}",
                &current_exe().unwrap().to_string_lossy(),
                self.to_args()
            );

            process::platform::run_process_as_current_user(&self.label, &cmd, false).await?;
        }

        let this = self.clone();
        tokio::task::spawn_blocking(move || this.show_blocking())
            .await
            .map_err(|e| io::Error::other(e.to_string()))?
    }
}

impl BlockingNotification for MessageDialog<Alert> {
    type Output = ();

    fn show_blocking(&self) -> io::Result<Self::Output> {
        DialogBuilder::message()
            .set_title(&self.title)
            .set_text(&self.text)
            .set_level(self.message_level)
            .alert()
            .show()
            .map_err(|e| match e {
                native_dialog::Error::Io(e) => e,
                e => io::Error::other(e.to_string()),
            })
    }
}

#[async_trait]
impl AsyncNotification for MessageDialog<Confirm> {
    type Output = bool;

    async fn show(&self) -> io::Result<Self::Output> {
        let is_admin = matches!(
            env::var(process::get_admin_var(&self.label))
                .map(|v| v.to_lowercase())
                .as_deref(),
            Ok("1" | "true")
        );

        if is_admin || cfg!(target_os = "macos") {
            let cmd = format!(
                "{} confirm {}",
                &current_exe().unwrap().to_string_lossy(),
                self.to_args()
            );

            let output =
                process::platform::run_process_as_current_user(&self.label, &cmd, false).await?;
            let output = output.trim();

            return if output == "true" {
                Ok(true)
            } else if output == "false" {
                Ok(false)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid output from subprocess: {output}"),
                ))
            };
        }
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.show_blocking())
            .await
            .map_err(|e| io::Error::other(e.to_string()))?
    }
}

impl BlockingNotification for MessageDialog<Confirm> {
    type Output = bool;

    fn show_blocking(&self) -> io::Result<Self::Output> {
        DialogBuilder::message()
            .set_title(&self.title)
            .set_text(&self.text)
            .set_level(self.message_level)
            .confirm()
            .show()
            .map_err(|e| match e {
                native_dialog::Error::Io(e) => e,
                e => io::Error::other(e.to_string()),
            })
    }
}
