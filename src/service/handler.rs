#[cfg(feature = "async-tokio")]
use futures::Future;
#[cfg(feature = "async-tokio")]
use std::pin::Pin;

#[maybe_async::async_impl]
pub type StopHandler = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[maybe_async::sync_impl]
pub type StopHandler = Box<dyn Fn() + Send>;

#[maybe_async::maybe_async]
pub trait Handler {
    fn new() -> Self;
    fn get_service_name<'a>() -> &'a str;
    fn get_stop_handler(&mut self) -> StopHandler;
    async fn run_service<F: FnOnce() + Send>(self, on_started: F) -> u32;
}
