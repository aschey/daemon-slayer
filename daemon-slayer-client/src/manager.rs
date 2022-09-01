use crate::Result;
use crate::{Builder, Status};
pub trait Manager {
    fn builder(name: impl Into<String>) -> Builder;
    fn new(name: impl Into<String>) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn from_builder(builder: Builder) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn display_name(&self) -> &str;
    fn description(&self) -> &str;
    fn args(&self) -> &Vec<String>;
    fn install(&self) -> Result<()>;
    fn uninstall(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn query_status(&self) -> Result<Status>;
}
