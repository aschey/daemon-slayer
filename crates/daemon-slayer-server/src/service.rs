use futures::Future;

use crate::{Handler, ServiceError};

pub trait Service: Handler {
    fn run_as_service(
        input_data: Option<Self::InputData>,
    ) -> impl Future<Output = Result<(), ServiceError<Self::Error>>> + Send;

    fn run_directly(
        input_data: Option<Self::InputData>,
    ) -> impl Future<Output = Result<(), ServiceError<Self::Error>>> + Send;
}
