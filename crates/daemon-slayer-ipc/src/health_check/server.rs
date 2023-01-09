use std::time::Duration;

use daemon_slayer_core::{async_trait, server::ServiceContext, BoxedError, FutureExt};
use futures::StreamExt;
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};

use crate::get_socket_address;

pub struct Server {
    pub(crate) sock_path: String,
}

impl Server {
    pub fn new(app_name: String) -> Self {
        let sock_path = get_socket_address(&app_name, "health");
        Self { sock_path }
    }
}

#[async_trait]
impl daemon_slayer_core::server::BackgroundService for Server {
    fn name<'a>() -> &'a str {
        "ipc_health_check_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut endpoint = Endpoint::new(self.sock_path);
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        let incoming = endpoint.incoming().unwrap();
        futures::pin_mut!(incoming);
        let mut buf = [0u8; 256];

        while let Ok(Some(result)) = incoming
            .next()
            .cancel_on_shutdown(&context.cancellation_token())
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
        Ok(())
    }
}
