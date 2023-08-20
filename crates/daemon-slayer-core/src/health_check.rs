use crate::BoxedError;

#[async_trait::async_trait]
pub trait HealthCheck {
    async fn invoke(&mut self) -> Result<(), BoxedError>;
}
