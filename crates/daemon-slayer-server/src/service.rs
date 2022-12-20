use std::error::Error;

use daemon_slayer_core::BoxedError;

use crate::Handler;

#[async_trait::async_trait]
pub trait Service: Handler {
    async fn run_as_service(input_data: Option<Self::InputData>) -> Result<(), BoxedError>;

    async fn run_directly(input_data: Option<Self::InputData>) -> Result<(), BoxedError>;
}
