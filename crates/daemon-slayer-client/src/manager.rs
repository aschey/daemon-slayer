use daemon_slayer_core::Label;
use dyn_clonable::clonable;

use crate::config::Builder;
use crate::{Info, Result};

#[clonable]
pub trait Manager: Clone + Send + Sync + 'static {
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
