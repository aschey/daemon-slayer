use std::error::Error;
#[cfg(feature = "async-tokio")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(feature = "ipc-health-check")]
#[cfg(feature = "async-tokio")]
pub struct IpcHealthCheckAsync {
    sock_path: String,
    read_buf: [u8; 256],
}

#[cfg(feature = "ipc-health-check")]
impl IpcHealthCheckAsync {
    pub fn new(sock_path: impl Into<String>) -> Self {
        Self {
            sock_path: sock_path.into(),
            read_buf: [0u8; 256],
        }
    }
}

#[cfg(feature = "ipc-health-check")]
#[async_trait::async_trait]
impl super::HealthCheckAsync for IpcHealthCheckAsync {
    async fn invoke(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut client = parity_tokio_ipc::Endpoint::connect(&self.sock_path).await?;
        let _ = client.write_u8(0).await?;

        let _ = client.read(&mut self.read_buf).await?;
        Ok(())
    }
}
