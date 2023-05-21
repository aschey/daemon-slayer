use std::{
    env::{self, current_exe},
    io,
};

use notify_rust::{Hint, Timeout, Urgency};

use crate::{process::get_spawn_interactive_var, Label};

#[derive(thiserror::Error, Debug)]
pub enum NotificationError {
    #[error("Failed to create notification: {0}")]
    NotificationFailure(notify_rust::error::Error),
    #[error("Failed to spawn notification process: {0}")]
    ProcessSpawnFailure(io::Error),
}

pub struct Notification {
    inner: notify_rust::Notification,
    image_path: Option<String>,
    sound_name: Option<String>,
    pub label: Label,
}

impl Notification {
    pub fn new(label: Label) -> Self {
        let mut inner = notify_rust::Notification::default();
        inner.appname(&label.application);
        // TODO: set app id in windows when the app is installed
        // https://learn.microsoft.com/en-us/windows/win32/shell/appids
        // inner.app_id(app_id)

        Self {
            inner,
            label,
            image_path: None,
            sound_name: None,
        }
    }

    pub fn summary(mut self, summary: impl AsRef<str>) -> Self {
        self.inner.summary(summary.as_ref());
        self
    }

    pub fn subtitle(mut self, subtitle: impl AsRef<str>) -> Self {
        self.inner.subtitle(subtitle.as_ref());
        self
    }

    pub fn image_path(mut self, path: impl AsRef<str>) -> Self {
        self.image_path = Some(path.as_ref().to_owned());
        #[cfg(not(target_os = "macos"))]
        self.inner.image_path(path.as_ref());
        self
    }

    pub fn sound_name(mut self, name: impl AsRef<str>) -> Self {
        self.sound_name = Some(name.as_ref().to_owned());
        self.inner.sound_name(name.as_ref());
        self
    }

    pub fn body(mut self, body: impl AsRef<str>) -> Self {
        self.inner.body(body.as_ref());
        self
    }

    pub fn icon(mut self, icon: impl AsRef<str>) -> Self {
        self.inner.icon(icon.as_ref());
        self
    }

    pub fn auto_icon(mut self) -> Self {
        self.inner.auto_icon();
        self
    }

    #[cfg_attr(any(windows, target_os = "macos"), allow(unused_variables, unused_mut))]
    pub fn hint(mut self, hint: impl Into<Hint>) -> Self {
        #[cfg(all(unix, not(target_os = "macos")))]
        self.inner.hint(hint.into());
        self
    }

    pub fn timeout(mut self, timeout: impl Into<Timeout>) -> Self {
        self.inner.timeout(timeout);
        self
    }

    #[cfg_attr(any(windows, target_os = "macos"), allow(unused_variables, unused_mut))]
    pub fn urgency(mut self, urgency: impl Into<Urgency>) -> Self {
        #[cfg(all(unix, not(target_os = "macos")))]
        self.inner.urgency(urgency.into());
        self
    }

    pub fn action(mut self, identifier: impl AsRef<str>, label: impl AsRef<str>) -> Self {
        self.inner.action(identifier.as_ref(), label.as_ref());
        self
    }

    pub fn id(mut self, id: u32) -> Self {
        self.inner.id(id);
        self
    }

    pub async fn show(&self) -> Result<(), NotificationError> {
        // Windows services running as admin can't send notifications
        // We get around this by spawning a separate process running as the current user
        // and sending the notification from there
        #[cfg(windows)]
        if let Ok("1" | "true") = env::var(get_spawn_interactive_var(&self.label))
            .map(|v| v.to_lowercase())
            .as_deref()
        {
            let cmd = format!(
                "{} notify {}",
                &current_exe().unwrap().to_string_lossy(),
                self.to_args()
            );

            return crate::process::windows::start_process_as_current_user(&cmd, false)
                .map_err(NotificationError::ProcessSpawnFailure);
        }

        #[cfg(target_os = "linux")]
        if let Ok("") | Err(_) = env::var("DBUS_SESSION_BUS_ADDRESS").as_deref() {
            // If the session bus address is not set, we can't spawn notifications from the current process (we're probably running as root)
            // Spawn the process as the logged in users instead
            let cmd = format!(
                "{} notify {}",
                &current_exe().unwrap().to_string_lossy(),
                self.to_args()
            );
            crate::process::linux::run_process_as_logged_on_users(&cmd).await;
            return Ok(());
        }

        self.inner.show_async().await.unwrap();

        Ok(())
    }

    fn to_args(&self) -> String {
        let mut args = format!("\"{}\"", self.inner.summary);
        if let Some(subtitle) = &self.inner.subtitle {
            args += &format!(" -s \"{subtitle}\"");
        }
        if !self.inner.body.is_empty() {
            args += &format!(" -b \"{}\"", self.inner.body);
        }
        if let Some(image_path) = &self.image_path {
            args += &format!(" -i \"{image_path}\"");
        }
        if let Some(sound) = &self.sound_name {
            args += &format!(" --sound-name \"{sound}\"");
        }
        if !self.inner.icon.is_empty() {
            args += &format!(" --icon \"{}\"", self.inner.icon);
        }

        args
    }
}
