use crate::{Info, Result};
use dyn_clonable::clonable;

#[clonable]
pub trait Manager: Clone + Send + Sync + 'static {
    fn name(&self) -> String;
    fn display_name(&self) -> &str;
    fn description(&self) -> &str;
    fn arguments(&self) -> &Vec<String>;
    fn reload_configuration(&self) -> Result<()>;
    fn on_configuration_changed(&mut self) -> Result<()>;
    fn install(&self) -> Result<()>;
    fn uninstall(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn restart(&self) -> Result<()>;
    fn enable_autostart(&mut self) -> Result<()>;
    fn disable_autostart(&mut self) -> Result<()>;
    fn info(&self) -> Result<Info>;
}
