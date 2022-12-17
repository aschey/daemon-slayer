use std::error::Error;

#[async_trait::async_trait]
pub trait HealthCheck {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
}
