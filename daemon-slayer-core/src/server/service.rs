#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait Service: Send {
    type Builder;
    type Client;

    async fn run_service(builder: Self::Builder) -> Self;

    fn get_client(&mut self) -> Self::Client;

    async fn stop(self);
}
