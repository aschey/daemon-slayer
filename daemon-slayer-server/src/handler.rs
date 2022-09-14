#[cfg(feature = "async-tokio")]
use futures::Future;
#[cfg(feature = "async-tokio")]
use std::pin::Pin;
use std::{error::Error, path::PathBuf};

use crate::Event;

#[cfg(feature = "async-tokio")]
pub type EventHandlerAsync = Box<
    dyn Fn(Event) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

#[cfg(feature = "blocking")]
pub type EventHandlerSync = Box<dyn Fn(Event) -> Result<(), Box<dyn Error + Send + Sync>> + Send>;

#[maybe_async_cfg::maybe(
    idents(EventHandler),
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait Handler {
    fn new() -> Self;
    fn get_service_name<'a>() -> &'a str;
    fn get_watch_paths(&self) -> Vec<PathBuf> {
        vec![]
    }
    fn get_event_handler(&mut self) -> EventHandler;
    async fn run_service<F: FnOnce() + Send>(
        self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
