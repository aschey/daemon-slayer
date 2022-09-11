use std::error::Error;

#[cfg(feature = "grpc-health-check")]
mod grpc;
#[cfg(feature = "http-health-check")]
mod http;
#[cfg(feature = "ipc-health-check")]
mod ipc;

#[cfg(feature = "grpc-health-check")]
pub use grpc::*;
#[cfg(feature = "http-health-check")]
pub use http::*;
#[cfg(feature = "ipc-health-check")]
pub use ipc::*;

#[maybe_async_cfg::maybe(
    idents(Service, HealthCheck),
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait HealthCheck {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
}
