use std::error::Error;

use daemon_slayer_core::health_check::HealthCheck;

pub struct GrpcHealthCheckAsync {
    endpoint: tonic::transport::Endpoint,
}

impl GrpcHealthCheckAsync {
    pub fn new<D>(endpoint: D) -> Result<Self, Box<dyn Error + Send + Sync>>
    where
        D: std::convert::TryInto<tonic::transport::Endpoint>,
        D::Error: std::error::Error + Send + Sync + 'static,
    {
        Ok(Self {
            endpoint: endpoint.try_into()?,
        })
    }
}

#[cfg(feature = "grpc-health-check")]
#[async_trait::async_trait]
impl HealthCheck for GrpcHealthCheckAsync {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
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
