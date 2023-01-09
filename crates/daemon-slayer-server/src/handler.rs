use daemon_slayer_core::{async_trait, server::ServiceContext, Label};
use std::{fmt, time::Duration};

#[async_trait]
pub trait Handler: Sized + Send + Sync + 'static {
    type InputData: Clone + Send + Sync + 'static;
    type Error: fmt::Debug + Send + Sync + 'static;

    async fn new(
        context: ServiceContext,
        input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error>;

    fn shutdown_timeout() -> Duration {
        Duration::from_secs(5)
    }

    fn label() -> Label;

    async fn run_service<F: FnOnce() + Send>(self, on_started: F) -> Result<(), Self::Error>;
}
