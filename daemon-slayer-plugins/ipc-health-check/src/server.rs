use std::{error::Error, time::Duration};

use crate::{builder::Builder, client::Client};
use daemon_slayer_core::server::{FutureExt, SubsystemHandle};
use futures::StreamExt;
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt},
    task::JoinHandle,
};

pub struct Server {
    handle: tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>,
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for Server {
    type Builder = Builder;

    type Client = Client;

    async fn run_service(builder: Self::Builder, subsys: SubsystemHandle) -> Self {
        let handle = tokio::spawn(async move {
            let mut endpoint = Endpoint::new(builder.sock_path);
            endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create()?);

            let incoming = endpoint.incoming()?;
            futures::pin_mut!(incoming);
            let mut buf = [0u8; 256];

            while let Ok(Some(result)) = incoming.next().cancel_on_shutdown(&subsys).await {
                match result {
                    Ok(stream) => {
                        let (mut reader, mut writer) = split(stream);

                        let _ = reader.read(&mut buf).await?;
                        writer.write_all(b"healthy").await?;
                    }
                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }

            Ok(())
        });

        Self { handle }
    }

    fn get_client(&mut self) -> Self::Client {
        Client {}
    }

    async fn stop(self) {
        self.handle.await.unwrap();
    }
}
