use std::error::Error;

#[async_trait::async_trait]
pub trait Service {
    async fn run_service_main() -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn run_service_direct() -> Result<(), Box<dyn Error + Send + Sync>>;
}
