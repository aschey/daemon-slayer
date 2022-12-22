use daemon_slayer_core::{server::ServiceContext, Label};
use std::{fmt, time::Duration};

#[async_trait::async_trait]
pub trait Handler {
    type InputData: Clone + Send + Sync + 'static;
    type Error: fmt::Debug + Send + Sync + 'static;

    async fn new(context: ServiceContext, input_data: Option<Self::InputData>) -> Self;

    fn shutdown_timeout() -> Duration {
        Duration::from_secs(5)
    }

    fn label() -> Label;

    async fn run_service<F: FnOnce() + Send>(self, on_started: F) -> Result<(), Self::Error>;
}
