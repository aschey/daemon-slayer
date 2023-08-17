use std::fmt;

use daemon_slayer_core::server::error::BackgroundServiceErrors;
use daemon_slayer_core::BoxedError;

#[derive(thiserror::Error, Debug)]
pub enum ServiceError<E: fmt::Debug + Send + Sync + 'static> {
    #[error("Error executing service: {0:?}. Background service failures: {1:?}")]
    ExecutionFailure(E, Option<BackgroundServiceErrors>),
    #[error("{0:?}")]
    BackgroundServiceFailure(BackgroundServiceErrors),
    #[error("Service manager failed during initialization: {0}: {1:?}")]
    InitializationFailure(String, #[source] BoxedError),
}

impl<E: fmt::Debug + Send + Sync> ServiceError<E> {
    pub(crate) fn from_service_result(
        service_result: Result<(), E>,
        background_service_errors: Result<(), BackgroundServiceErrors>,
    ) -> Result<(), Self> {
        match (service_result, background_service_errors) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(service_errors)) => {
                Err(ServiceError::BackgroundServiceFailure(service_errors))
            }
            (Err(e), Ok(())) => Err(ServiceError::ExecutionFailure(e, None)),
            (Err(e), Err(service_errors)) => {
                Err(ServiceError::ExecutionFailure(e, Some(service_errors)))
            }
        }
    }
}
