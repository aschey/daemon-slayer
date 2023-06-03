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
    async fn reload_config(&mut self) -> Result<(), io::Error>;
    async fn on_config_changed(&mut self) -> Result<(), io::Error>;
    async fn install(&self) -> Result<(), io::Error>;
    async fn uninstall(&self) -> Result<(), io::Error>;
    async fn start(&self) -> Result<(), io::Error>;
    async fn stop(&self) -> Result<(), io::Error>;
    async fn restart(&self) -> Result<(), io::Error>;
    async fn enable_autostart(&mut self) -> Result<(), io::Error>;
    async fn disable_autostart(&mut self) -> Result<(), io::Error>;
    async fn info(&self) -> Result<Info, io::Error>;
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

    pub async fn reload_config(&mut self) -> Result<(), io::Error> {
        self.inner.reload_config().await
    }

    pub async fn install(&self) -> Result<(), io::Error> {
        self.inner.install().await
    }

    pub async fn uninstall(&self) -> Result<(), io::Error> {
        self.inner.uninstall().await
    }

    pub async fn start(&self) -> Result<(), io::Error> {
        self.inner.start().await
    }

    pub async fn stop(&self) -> Result<(), io::Error> {
        self.inner.stop().await
    }

    pub async fn restart(&self) -> Result<(), io::Error> {
        self.inner.restart().await
    }

    pub async fn enable_autostart(&mut self) -> Result<(), io::Error> {
        self.inner.enable_autostart().await
    }

    pub async fn disable_autostart(&mut self) -> Result<(), io::Error> {
        self.inner.disable_autostart().await
    }

    pub async fn info(&self) -> Result<Info, io::Error> {
        self.inner.info().await
    }
}

#[async_trait]
impl ConfigWatcher for ServiceManager {
    async fn on_config_changed(&mut self) -> Result<(), io::Error> {
        self.inner.on_config_changed().await
    }
}
