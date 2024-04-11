#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "config")]
pub mod config;
#[cfg(feature = "health-check")]
pub mod health_check;
mod label;
#[cfg(feature = "notify")]
pub mod notify;
#[cfg(feature = "process")]
pub mod process;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "signal")]
pub mod signal;
pub mod socket_activation;

use std::any::Any;
use std::error::Error;
use std::fmt::Display;
use std::str::FromStr;

#[cfg(feature = "daemon-slayer-macros")]
pub use daemon_slayer_macros::*;
pub use futures_cancel::*;
pub use label::*;
pub use tokio_util::sync::CancellationToken;

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> AsAny for T
where
    T: Any,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub type BoxedError = Box<dyn Error + Send + Sync + 'static>;

#[derive(thiserror::Error, Debug)]
pub enum CommandArgParseError {
    #[error("Short argument {0} should only have a single character")]
    InvalidShortArg(String),
    #[error("Argument should not be empty")]
    EmptyArg,
}

#[derive(Clone, Debug)]
pub enum CommandArg {
    Subcommand(String),
    ShortArg(char),
    LongArg(String),
}

impl Display for CommandArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subcommand(val) => write!(f, "{val}"),
            Self::ShortArg(val) => write!(f, "-{val}"),
            Self::LongArg(val) => write!(f, "--{val}"),
        }
    }
}

impl FromStr for CommandArg {
    type Err = CommandArgParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_owned();
        if s.starts_with("--") {
            Ok(CommandArg::LongArg(s.replacen("--", "", 1)))
        } else if s.starts_with('-') {
            // '-' plus single character
            if s.len() == 2 {
                Ok(CommandArg::ShortArg(
                    s.chars()
                        .nth(1)
                        .expect("Length should already be validated"),
                ))
            } else {
                Err(CommandArgParseError::InvalidShortArg(s))
            }
        } else if s.is_empty() {
            Err(CommandArgParseError::EmptyArg)
        } else {
            Ok(CommandArg::Subcommand(s))
        }
    }
}
