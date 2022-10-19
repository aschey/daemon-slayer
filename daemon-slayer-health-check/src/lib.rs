#[cfg(feature = "grpc-health-check")]
mod grpc;
#[cfg(feature = "http-health-check")]
mod http;
#[cfg(feature = "ipc-health-check")]
mod ipc;

#[cfg(all(feature = "async-tokio", feature = "grpc-health-check"))]
pub use grpc::GrpcHealthCheckAsync as GrpcHealthCheck;
#[cfg(all(feature = "async-tokio", feature = "http-health-check"))]
pub use http::HttpHealthCheckAsync as HttpHealthCheck;
#[cfg(all(feature = "async-tokio", feature = "ipc-health-check"))]
pub use ipc::IpcHealthCheckAsync as IpcHealthCheck;

#[cfg(feature = "blocking")]
pub mod blocking {
    #[cfg(feature = "http-health-check")]
    pub use crate::http::HttpHealthCheckSync as HttpHealthCheck;
}
