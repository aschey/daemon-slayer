use std::error::Error;

#[cfg(feature = "async-tokio")]
#[async_trait::async_trait]
pub trait HealthCheck {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub mod blocking {
    use std::error::Error;

    pub trait HealthCheck {
        fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;
    }
}
