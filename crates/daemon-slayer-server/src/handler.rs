use std::fmt;

use daemon_slayer_core::server::background_service::{self, ServiceContext};
use daemon_slayer_core::{async_trait, Label};

#[async_trait]
pub trait Handler: Sized + Send + Sync + 'static {
    type InputData: Clone + Send + Sync + 'static;
    type Error: fmt::Debug + Send + Sync + 'static;

    async fn new(
        context: ServiceContext,
        input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error>;

    fn background_service_settings() -> background_service::Settings {
        background_service::Settings::default()
    }

    fn label() -> Label;

    async fn run_service<F: FnOnce() + Send>(self, notify_ready: F) -> Result<(), Self::Error>;
}
