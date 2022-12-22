use daemon_slayer_core::{health_check::HealthCheck, BoxedError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub struct IpcHealthCheck {
    sock_path: String,
    read_buf: [u8; 256],
}

impl IpcHealthCheck {
    pub fn new(app_name: impl Into<String>) -> Self {
        let app_name = app_name.into();
        #[cfg(unix)]
        let sock_path = format!("/tmp/{app_name}_health.sock");
        #[cfg(windows)]
        let sock_path = format!("\\\\.\\pipe\\{app_name}_health");

        Self {
            sock_path,
            read_buf: [0u8; 256],
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for IpcHealthCheck {
    async fn invoke(&mut self) -> Result<(), BoxedError> {
        let mut client = parity_tokio_ipc::Endpoint::connect(&self.sock_path).await?;
        let _ = client.write_u8(0).await?;

        let _ = client.read(&mut self.read_buf).await?;
        Ok(())
    }
}
