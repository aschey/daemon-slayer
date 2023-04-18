use daemon_slayer_core::{async_trait, health_check::HealthCheck, BoxedError};
use std::error::Error;
use tonic_health::pb::{
    health_check_response::ServingStatus, health_client::HealthClient, HealthCheckRequest,
};

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
