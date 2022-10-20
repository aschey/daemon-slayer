use std::error::Error;

use crate::service_context::ServiceContext;

#[async_trait::async_trait]
pub trait Handler {
    async fn new(context: &mut ServiceContext) -> Self;
    fn get_service_name<'a>() -> &'a str;

    async fn run_service<F: FnOnce() + Send>(
        self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
