mod platform;
use std::{error::Error, result};

pub use platform::ServiceManager;
mod config;
pub use config::Builder;
mod level;
pub use level::Level;
mod manager;
pub use manager::Manager;
mod status;
pub use status::Status;

pub type Result<T> = result::Result<T, Box<dyn Error>>;
