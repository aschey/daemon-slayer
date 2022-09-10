use std::{error::Error, time::Duration};

use futures::StreamExt;
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt},
    task::JoinHandle,
};

pub struct IpcHealthCheckServer {
    sock_path: String,
}

impl IpcHealthCheckServer {
    pub fn new(app_name: &str) -> Self {
        #[cfg(unix)]
        let sock_path = format!("/tmp/{app_name}health.sock");
        #[cfg(windows)]
        let sock_path = format!("\\\\.\\pipe\\{app_name}health");
        Self { sock_path }
    }

    pub fn sock_path(&self) -> &str {
        &self.sock_path
    }

    pub fn spawn_server(&self) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
        let sock_path = self.sock_path.clone();
        tokio::spawn(async {
            let mut endpoint = Endpoint::new(sock_path);
            endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create()?);

            let incoming = endpoint.incoming()?;
            futures::pin_mut!(incoming);
            let mut buf = [0u8; 256];
            while let Some(result) = incoming.next().await {
                match result {
                    Ok(stream) => {
                        let (mut reader, mut writer) = split(stream);

                        let _ = reader.read(&mut buf).await?;
                        writer.write_all(b"healthy").await?;
                    }
                    _ => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }

            Ok(())
        })
    }
}
