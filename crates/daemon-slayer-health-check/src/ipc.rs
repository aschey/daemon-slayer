use std::io;
use std::path::PathBuf;

use async_trait::async_trait;
use daemon_slayer_core::BoxedError;
use daemon_slayer_core::health_check::HealthCheck;
use tipsy::{Endpoint, IntoIpcPath, ServerId};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub struct IpcHealthCheck {
    sock_path: PathBuf,
    read_buf: [u8; 256],
}

impl IpcHealthCheck {
    pub fn new(app_name: impl Into<String>) -> io::Result<Self> {
        let sock_path = ServerId::new(format!("{}_health", app_name.into()))
            .parent_folder("/tmp")
            .into_ipc_path()?;

        Ok(Self {
            sock_path,
            read_buf: [0u8; 256],
        })
    }
}

#[async_trait]
impl HealthCheck for IpcHealthCheck {
    async fn invoke(&mut self) -> Result<(), BoxedError> {
        let mut client = Endpoint::connect(self.sock_path.clone()).await?;
        let _ = client.write_u8(0).await?;

        let _ = client.read(&mut self.read_buf).await?;
        Ok(())
    }
}
