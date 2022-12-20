use std::{error::Error, time::Duration};

use daemon_slayer_core::{server::ServiceContext, Label};

#[async_trait::async_trait]
pub trait Handler {
    type InputData: Clone + Send + Sync + 'static;

    async fn new(context: ServiceContext, input_data: Option<Self::InputData>) -> Self;

    fn shutdown_timeout() -> Duration {
        Duration::from_secs(5)
    }

    fn label() -> Label;

    async fn run_service<F: FnOnce() + Send>(
        self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
