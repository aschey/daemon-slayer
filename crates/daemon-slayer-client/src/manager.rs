use crate::{config::Config, Info};
use daemon_slayer_core::{async_trait, config::ConfigWatcher, Label};
use dyn_clonable::clonable;
use std::{io, result::Result};

#[clonable]
#[async_trait]
pub(crate) trait Manager: std::fmt::Debug + Clone + Send + Sync + 'static {
    fn name(&self) -> String;
    fn display_name(&self) -> &str;
    fn label(&self) -> &Label;
    fn description(&self) -> &str;
    fn arguments(&self) -> &Vec<String>;
    fn config(&self) -> Config;
    async fn status_command(&self) -> io::Result<Command>;
    async fn reload_config(&mut self) -> io::Result<()>;
    async fn on_config_changed(&mut self) -> io::Result<()>;
    async fn install(&self) -> io::Result<()>;
    async fn uninstall(&self) -> io::Result<()>;
    async fn start(&self) -> io::Result<()>;
    async fn stop(&self) -> io::Result<()>;
    async fn restart(&self) -> io::Result<()>;
    async fn enable_autostart(&mut self) -> io::Result<()>;
    async fn disable_autostart(&mut self) -> io::Result<()>;
    async fn info(&self) -> io::Result<Info>;
}

pub struct Command {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ServiceManager {
    inner: Box<dyn Manager>,
}

impl ServiceManager {
    pub(crate) fn new(inner: impl Manager) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
    pub fn name(&self) -> String {
        self.inner.name()
    }

    pub fn display_name(&self) -> &str {
        self.inner.display_name()
    }

    pub fn label(&self) -> &Label {
        self.inner.label()
    }

    pub fn config(&self) -> Config {
        self.inner.config()
    }

    pub fn description(&self) -> &str {
        self.inner.description()
    }

    pub fn arguments(&self) -> &Vec<String> {
        self.inner.arguments()
    }

    pub async fn status_command(&self) -> io::Result<Command> {
        self.inner.status_command().await
    }

    pub async fn reload_config(&mut self) -> io::Result<()> {
        self.inner.reload_config().await
    }

    pub async fn install(&self) -> io::Result<()> {
        self.inner.install().await
    }

    pub async fn uninstall(&self) -> io::Result<()> {
        self.inner.uninstall().await
    }

    pub async fn start(&self) -> io::Result<()> {
        self.inner.start().await
    }

    pub async fn stop(&self) -> io::Result<()> {
        self.inner.stop().await
    }

    pub async fn restart(&self) -> io::Result<()> {
        self.inner.restart().await
    }

    pub async fn enable_autostart(&mut self) -> io::Result<()> {
        self.inner.enable_autostart().await
    }

    pub async fn disable_autostart(&mut self) -> io::Result<()> {
        self.inner.disable_autostart().await
    }

    pub async fn info(&self) -> io::Result<Info> {
        self.inner.info().await
    }
}

#[async_trait]
impl ConfigWatcher for ServiceManager {
    async fn on_config_changed(&mut self) -> io::Result<()> {
        self.inner.on_config_changed().await
    }
}
