#[cfg(feature = "async-tokio")]
use futures::Future;
#[cfg(feature = "async-tokio")]
use std::pin::Pin;

#[cfg(feature = "async-tokio")]
pub type StopHandlerAsync = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[cfg(feature = "blocking")]
pub type StopHandlerSync = Box<dyn Fn() + Send>;

#[maybe_async_cfg::maybe(
    idents(StopHandler),
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait Handler {
    fn new() -> Self;
    fn get_service_name<'a>() -> &'a str;
    fn get_stop_handler(&mut self) -> StopHandler;
    async fn run_service<F: FnOnce() + Send>(self, on_started: F) -> u32;
}
