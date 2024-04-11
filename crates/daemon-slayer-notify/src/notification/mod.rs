use std::env::{self, current_exe};
use std::io;

use async_trait::async_trait;
use daemon_slayer_core::{process, Label};
use notify_rust::{Hint, Timeout, Urgency};
use tap::TapFallible;
use tracing::error;

use super::AsyncNotification;

#[cfg(feature = "cli")]
pub mod cli;

#[derive(Clone)]
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

#[async_trait]
impl AsyncNotification for Notification {
    type Output = ();
    async fn show(&self) -> io::Result<Self::Output> {
        // Services running as admin can't send notifications
        // We get around this by spawning a separate process running as the current user
        // and sending the notification from there

        if let Ok("1" | "true") = env::var(process::get_admin_var(&self.label))
            .map(|v| v.to_lowercase())
            .as_deref()
        {
            let cmd = format!(
                "{} notify {}",
                &current_exe().unwrap().to_string_lossy(),
                self.to_args()
            );

            process::platform::run_process_as_current_user(&self.label, &cmd, false)
                .await
                .tap_err(|e| error!("Error spawning notification process: {e:?}"))
                .ok();

            return Ok(());
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        self.inner
            .show_async()
            .await
            .tap_err(|e| error!("Error showing notification: {e:?}"))
            .ok();

        #[cfg(any(not(unix), target_os = "macos"))]
        self.inner
            .show()
            .tap_err(|e| error!("Error showing notification: {e:?}"))
            .ok();
        Ok(())
    }
}
