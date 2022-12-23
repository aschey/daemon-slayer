use crate::Info;
use dyn_clonable::clonable;
use std::{io, result::Result};

#[clonable]
pub trait Manager: Clone + Send + Sync + 'static {
    fn name(&self) -> String;
    fn display_name(&self) -> &str;
    fn description(&self) -> &str;
    fn arguments(&self) -> &Vec<String>;
    fn reload_config(&self) -> Result<(), io::Error>;
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
