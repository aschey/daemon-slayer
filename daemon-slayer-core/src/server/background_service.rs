use tokio_graceful_shutdown::SubsystemHandle;

#[async_trait::async_trait]
pub trait BackgroundService: Send {
    type Builder;
    type Client;

    async fn run_service(builder: Self::Builder, subsys: SubsystemHandle) -> Self;

    fn get_client(&mut self) -> Self::Client;

    async fn stop(self);
}
