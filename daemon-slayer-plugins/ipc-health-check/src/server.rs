use std::{error::Error, time::Duration};

use crate::{builder::Builder, client::Client};
use futures::StreamExt;
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt},
    task::JoinHandle,
};

pub struct Server {
    handle: tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::Service for Server {
    type Builder = Builder;

    type Client = Client;

    async fn run_service(builder: Self::Builder) -> Self {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(32);
        let handle = tokio::spawn(async move {
            let mut endpoint = Endpoint::new(builder.sock_path);
            endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create()?);

            let incoming = endpoint.incoming()?;
            futures::pin_mut!(incoming);
            let mut buf = [0u8; 256];
            loop {
                tokio::select! {
                    result = incoming.next() => {
                        match result {
                            Some(Ok(stream)) => {
                                let (mut reader, mut writer) = split(stream);

                                let _ = reader.read(&mut buf).await?;
                                writer.write_all(b"healthy").await?;
                            }
                            Some(Err(_)) => {
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                            None => {
                                break;
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }

            Ok(())
        });

        Self {
            handle,
            shutdown_tx,
        }
    }

    fn get_client(&mut self) -> Self::Client {
        Client {}
    }

    async fn stop(self) {
        self.shutdown_tx.send(()).await;
        self.handle.await;
    }
}
