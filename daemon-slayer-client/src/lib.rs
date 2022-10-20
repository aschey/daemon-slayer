mod platform;
use std::{error::Error, result};

pub use platform::ServiceManager;
pub mod config;
mod level;
pub use level::Level;
mod manager;
pub use manager::*;
mod state;
pub use state::State;
mod info;
pub use info::Info;
pub type Result<T> = result::Result<T, Box<dyn Error + Send + Sync>>;
#[cfg(feature = "cli")]
pub mod cli;
