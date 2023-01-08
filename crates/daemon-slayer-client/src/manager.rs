use crate::Info;
use daemon_slayer_core::config::ConfigWatcher;
use dyn_clonable::clonable;
use std::{io, result::Result};

#[clonable]
pub(crate) trait Manager: Clone + Send + Sync + 'static {
    fn name(&self) -> String;
    fn display_name(&self) -> &str;
    fn description(&self) -> &str;
    fn arguments(&self) -> &Vec<String>;
    fn reload_config(&mut self) -> Result<(), io::Error>;
    fn on_config_changed(&mut self) -> Result<(), io::Error>;
    fn install(&self) -> Result<(), io::Error>;
    fn uninstall(&self) -> Result<(), io::Error>;
    fn start(&self) -> Result<(), io::Error>;
    fn stop(&self) -> Result<(), io::Error>;
    fn restart(&self) -> Result<(), io::Error>;
    fn enable_autostart(&mut self) -> Result<(), io::Error>;
    fn disable_autostart(&mut self) -> Result<(), io::Error>;
    fn info(&self) -> Result<Info, io::Error>;
}

#[derive(Clone)]
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

    pub fn description(&self) -> &str {
        self.inner.description()
    }

    pub fn arguments(&self) -> &Vec<String> {
        self.inner.arguments()
    }

    pub fn reload_config(&mut self) -> Result<(), io::Error> {
        self.inner.reload_config()
    }

    pub fn install(&self) -> Result<(), io::Error> {
        self.inner.install()
    }

    pub fn uninstall(&self) -> Result<(), io::Error> {
        self.inner.uninstall()
    }

    pub fn start(&self) -> Result<(), io::Error> {
        self.inner.start()
    }

    pub fn stop(&self) -> Result<(), io::Error> {
        self.inner.stop()
    }

    pub fn restart(&self) -> Result<(), io::Error> {
        self.inner.restart()
    }

    pub fn enable_autostart(&mut self) -> Result<(), io::Error> {
        self.inner.enable_autostart()
    }

    pub fn disable_autostart(&mut self) -> Result<(), io::Error> {
        self.inner.disable_autostart()
    }

    pub fn info(&self) -> Result<Info, io::Error> {
        self.inner.info()
    }
}

impl ConfigWatcher for ServiceManager {
    fn on_config_changed(&mut self) -> Result<(), io::Error> {
        self.inner.on_config_changed()
    }
}
