use daemon_slayer_core::health_check::HealthCheck;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub struct IpcHealthCheckAsync {
    sock_path: String,
    read_buf: [u8; 256],
}

impl IpcHealthCheckAsync {
    pub fn new(sock_path: impl Into<String>) -> Self {
        Self {
            sock_path: sock_path.into(),
            read_buf: [0u8; 256],
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for IpcHealthCheckAsync {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut client = parity_tokio_ipc::Endpoint::connect(&self.sock_path).await?;
        let _ = client.write_u8(0).await?;

        let _ = client.read(&mut self.read_buf).await?;
        Ok(())
    }
}
