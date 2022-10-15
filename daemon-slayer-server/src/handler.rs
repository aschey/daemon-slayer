#[cfg(feature = "async-tokio")]
use futures::Future;
use std::error::Error;
#[cfg(feature = "async-tokio")]
use std::pin::Pin;

#[maybe_async_cfg::maybe(
    idents(EventHandler, ServiceContext),
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait Handler {
    async fn new(context: &mut crate::ServiceContext) -> Self;
    fn get_service_name<'a>() -> &'a str;

    async fn run_service<F: FnOnce() + Send>(
        self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
