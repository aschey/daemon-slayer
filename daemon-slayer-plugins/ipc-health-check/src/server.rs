use std::{error::Error, time::Duration};

use crate::{builder::Builder, client::Client};
use daemon_slayer_core::server::{FutureExt, ServiceContext, SubsystemHandle};
use futures::StreamExt;
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt},
    task::JoinHandle,
};

pub struct Server {
    pub(crate) sock_path: String,
}

impl Server {
    pub fn new(app_name: String) -> Self {
        #[cfg(unix)]
        let sock_path = format!("/tmp/{app_name}_health.sock");
        #[cfg(windows)]
        let sock_path = format!("\\\\.\\pipe\\{app_name}_health");
        Self { sock_path }
    }
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for Server {
    type Client = Client;

    async fn run(self, context: ServiceContext) {
        let mut endpoint = Endpoint::new(self.sock_path);
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        let incoming = endpoint.incoming().unwrap();
        futures::pin_mut!(incoming);
        let mut buf = [0u8; 256];

        while let Ok(Some(result)) = incoming
            .next()
            .cancel_on_shutdown(&context.get_subsystem_handle())
            .await
        {
            match result {
                Ok(stream) => {
                    let (mut reader, mut writer) = split(stream);

                    let _ = reader.read(&mut buf).await.unwrap();
                    writer.write_all(b"healthy").await.unwrap();
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }

    async fn get_client(&mut self) -> Self::Client {
        Client {}
    }
}
