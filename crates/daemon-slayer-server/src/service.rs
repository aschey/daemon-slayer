use crate::Handler;
use daemon_slayer_core::server::BackgroundServiceErrors;
use std::fmt;

#[async_trait::async_trait]
pub trait Service: Handler {
    async fn run_as_service(
        input_data: Option<Self::InputData>,
    ) -> Result<(), ServiceError<Self::Error>>;

    async fn run_directly(
        input_data: Option<Self::InputData>,
    ) -> Result<(), ServiceError<Self::Error>>;
}

#[derive(thiserror::Error, Debug)]
pub enum ServiceError<E: fmt::Debug + Send + Sync> {
    #[error("")]
    TimedOut,
    #[error("")]
    ExecutionFailure(E, Option<BackgroundServiceErrors>),
    #[error("")]
    BackgroundServiceFailure(BackgroundServiceErrors),
}
