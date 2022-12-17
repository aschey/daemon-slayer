use std::{error::Error, fmt::Debug};

use daemon_slayer_core::server::ServiceContext;

#[async_trait::async_trait]
pub trait Handler {
    type InputData: Clone + Send + Sync + 'static;
    async fn new(context: ServiceContext, input_data: Option<Self::InputData>) -> Self;
    fn get_service_name<'a>() -> &'a str;

    async fn run_service<F: FnOnce() + Send>(
        self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
