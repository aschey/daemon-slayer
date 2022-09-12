#[cfg(feature = "async-tokio")]
use futures::Future;
use std::error::Error;
#[cfg(feature = "async-tokio")]
use std::pin::Pin;

use crate::Event;

#[cfg(feature = "async-tokio")]
pub type EventHandlerAsync = Box<
    dyn Fn(Event) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

#[cfg(feature = "blocking")]
pub type EventHandlerSync = Box<dyn Fn(Event) -> Result<(), Box<dyn Error + Send + Sync>> + Send>;

#[cfg(feature = "async-tokio")]
#[async_trait::async_trait]
pub trait HandlerAsync {
    fn new() -> Self;
    fn get_service_name<'a>() -> &'a str;
    #[cfg(any(feature = "signal-handler-async", feature = "signal-handler-sync"))]
    fn get_event_handler(&mut self) -> EventHandlerAsync {
        Box::new(move |_| Box::pin(async { Ok(()) }))
    }
    async fn run_service<F: FnOnce() + Send>(
        self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

#[cfg(feature = "blocking")]
pub trait HandlerSync {
    fn new() -> Self;
    fn get_service_name<'a>() -> &'a str;
    #[cfg(any(feature = "signal-handler-async", feature = "signal-handler-sync"))]
    fn get_event_handler(&mut self) -> EventHandlerSync {
        Box::new(move |_| Ok(()))
    }
    fn run_service<F: FnOnce() + Send>(
        self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
