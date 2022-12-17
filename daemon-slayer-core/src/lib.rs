#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "health-check")]
pub mod health_check;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "signal")]
pub mod signal;

#[cfg(feature = "config")]
pub mod config;

mod app;
use std::any::Any;

pub use app::*;

#[cfg(feature = "daemon-slayer-macros")]
pub use daemon_slayer_macros::*;

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
