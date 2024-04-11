use std::error::Error;

use async_trait::async_trait;
use daemon_slayer_core::health_check::HealthCheck;
use daemon_slayer_core::BoxedError;
use tonic_health::pb::health_check_response::ServingStatus;
use tonic_health::pb::health_client::HealthClient;
use tonic_health::pb::HealthCheckRequest;

#[derive(Clone)]
pub struct GrpcHealthCheck {
    endpoint: tonic::transport::Endpoint,
}

impl GrpcHealthCheck {
    pub fn new<D>(endpoint: D) -> Result<Self, BoxedError>
    where
        D: std::convert::TryInto<tonic::transport::Endpoint>,
        D::Error: Error + Send + Sync + 'static,
    {
        Ok(Self {
            endpoint: endpoint.try_into()?,
        })
    }
}

#[async_trait]
impl HealthCheck for GrpcHealthCheck {
    async fn invoke(&mut self) -> Result<(), BoxedError> {
        let mut client = HealthClient::new(self.endpoint.connect().await?);

        let response = client.check(HealthCheckRequest::default()).await?;
        match response.into_inner().status() {
            ServingStatus::Serving => Ok(()),
            _ => Err("invalid status"),
        }?;

        Ok(())
    }
}
