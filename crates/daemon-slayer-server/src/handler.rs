use std::fmt;

use daemon_slayer_core::Label;
use daemon_slayer_core::server::background_service::{self, ServiceContext};
use futures::Future;

pub trait Handler: Sized + Send + Sync + 'static {
    type InputData: Clone + Send + Sync + 'static;
    type Error: fmt::Debug + Send + Sync + 'static;

    fn new(
        context: ServiceContext,
        input_data: Option<Self::InputData>,
    ) -> impl Future<Output = Result<Self, Self::Error>> + Send;

    fn background_service_settings() -> background_service::Settings {
        background_service::Settings::default()
    }

    fn label() -> Label;

    fn run_service<F: FnOnce() + Send>(
        self,
        notify_ready: F,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
