use crate::BoxedError;

use super::ServiceContext;
use std::time::Duration;

#[async_trait::async_trait]
pub trait BackgroundService: Send {
    type Client;

    fn shutdown_timeout() -> Duration {
        Duration::from_secs(1)
    }

    fn name<'a>() -> &'a str;

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError>;

    async fn get_client(&mut self) -> Self::Client;
}
