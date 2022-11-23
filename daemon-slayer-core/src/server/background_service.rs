use std::time::Duration;
use tokio_graceful_shutdown::SubsystemHandle;

#[async_trait::async_trait]
pub trait BackgroundService: Send {
    type Client;

    fn shutdown_timeout() -> Duration {
        Duration::from_secs(1)
    }

    async fn run(mut self, subsys: SubsystemHandle);

    async fn get_client(&mut self) -> Self::Client;
}
