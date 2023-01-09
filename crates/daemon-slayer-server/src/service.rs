use daemon_slayer_core::async_trait;

use crate::{Handler, ServiceError};

#[async_trait]
pub trait Service: Handler {
    async fn run_as_service(
        input_data: Option<Self::InputData>,
    ) -> Result<(), ServiceError<Self::Error>>;

    async fn run_directly(
        input_data: Option<Self::InputData>,
    ) -> Result<(), ServiceError<Self::Error>>;
}
