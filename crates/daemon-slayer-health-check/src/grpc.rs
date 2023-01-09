use daemon_slayer_core::{async_trait, health_check::HealthCheck, BoxedError};
use std::error::Error;
use tonic_health::proto::health_client::HealthClient;

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

        let response = client
            .check(tonic_health::proto::HealthCheckRequest::default())
            .await?;
        match response.into_inner().status() {
            tonic_health::proto::health_check_response::ServingStatus::Serving => Ok(()),
            _ => Err("invalid status"),
        }?;

        Ok(())
    }
}
