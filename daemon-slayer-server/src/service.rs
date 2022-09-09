use std::error::Error;

#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait Service {
    async fn run_service_main(self: Box<Self>) -> Result<(), Box<dyn Error>>;
    #[cfg(feature = "direct")]
    async fn run_service_direct(self: Box<Self>) -> Result<(), Box<dyn Error>>;
}
