use daemon_slayer_core::Label;

use crate::config::Builder;
use crate::{Info, Result};

pub trait Manager: Clone {
    fn builder(label: Label) -> Builder;
    fn new(name: Label) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn from_builder(builder: Builder) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn name(&self) -> String;
    fn display_name(&self) -> &str;
    fn description(&self) -> &str;
    fn args(&self) -> &Vec<String>;
    fn reload_configuration(&self) -> Result<()>;
    fn on_configuration_changed(&mut self) -> Result<()>;
    fn install(&self) -> Result<()>;
    fn uninstall(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn restart(&self) -> Result<()>;
    fn set_autostart_enabled(&mut self, enabled: bool) -> Result<()>;
    fn info(&self) -> Result<Info>;
}
