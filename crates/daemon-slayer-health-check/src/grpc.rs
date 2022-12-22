use daemon_slayer_core::{health_check::HealthCheck, BoxedError};
use std::error::Error;

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

#[async_trait::async_trait]
impl HealthCheck for GrpcHealthCheck {
    async fn invoke(&mut self) -> Result<(), BoxedError> {
        let mut client =
            tonic_health::proto::health_client::HealthClient::connect(self.endpoint.clone())
                .await?;
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
