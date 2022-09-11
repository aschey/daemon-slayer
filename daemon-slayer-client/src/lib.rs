mod platform;
use std::{error::Error, result};

pub use platform::ServiceManager;
mod config;
pub use config::Builder;
mod level;
pub use level::Level;
mod manager;
pub use manager::*;
mod state;
pub use state::State;
mod info;
pub use info::Info;
pub type Result<T> = result::Result<T, Box<dyn Error + Send + Sync>>;
pub mod health_check;
