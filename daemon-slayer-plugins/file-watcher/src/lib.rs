mod file_watcher_builder;
pub use file_watcher_builder::*;

mod file_watcher_command;

#[cfg(feature = "async-tokio")]
mod async_watcher;
#[cfg(feature = "async-tokio")]
pub use async_watcher::*;

#[cfg(feature = "blocking")]
mod blocking_watcher;
#[cfg(feature = "blocking")]
pub mod blocking {
    pub use crate::blocking_watcher::*;
}
