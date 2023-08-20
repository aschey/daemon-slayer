#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "grpc-health-check")]
mod grpc;
#[cfg(feature = "http-health-check")]
mod http;
#[cfg(feature = "ipc-health-check")]
mod ipc;

pub use daemon_slayer_core::health_check::HealthCheck;
#[cfg(feature = "grpc-health-check")]
pub use grpc::*;
#[cfg(feature = "http-health-check")]
pub use http::*;
#[cfg(feature = "ipc-health-check")]
pub use ipc::*;
