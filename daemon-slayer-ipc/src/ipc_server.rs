use std::{marker::PhantomData, mem, pin::Pin};

use bytes::{Bytes, BytesMut};
use daemon_slayer_core::server::{BackgroundService, FutureExt, SubsystemHandle};
use futures::{SinkExt, StreamExt};
use parity_tokio_ipc::Endpoint;
use serde::{Deserialize, Serialize};
use tarpc::serde_transport::Transport;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio_serde::{Deserializer, Serializer};

use crate::{
    build_transport, get_socket_address, ipc_client_stream::IpcClientStream, Codec, CodecWrapper,
    IpcClient, IpcRequestHandler,
};

pub struct IpcServer<H>
where
    H: IpcRequestHandler + 'static,
{
    endpoint: Endpoint,
    codec: Codec,
    handler: H,
    app_id: String,
}

impl<H> IpcServer<H>
where
    H: IpcRequestHandler + 'static,
{
    pub fn new(app_id: impl Into<String>, codec: Codec, handler: H) -> Self {
        let app_id = app_id.into();
        let endpoint = Endpoint::new(get_socket_address(&app_id, "ipc"));
        Self {
            app_id,
            endpoint,
            codec,
            handler,
        }
    }
}

#[async_trait::async_trait]
impl<H> BackgroundService for IpcServer<H>
where
    H: IpcRequestHandler + 'static,
{
    type Client = IpcClient<H::Req, H::Res>;

    async fn run(self, subsys: SubsystemHandle) {
        let incoming = self.endpoint.incoming().expect("failed to open new socket");
        futures::pin_mut!(incoming);

        while let Ok(Some(Ok(stream))) = incoming.next().cancel_on_shutdown(&subsys).await {
            let mut transport = build_transport(
                stream,
                CodecWrapper::<H::Req, H::Res>::new(self.codec.clone()),
            );

            let mut handler = self.handler.clone();
            tokio::spawn(async move {
                loop {
                    let req = transport.next().await.unwrap().unwrap();
                    let res = handler.handle_request(req).await;
                    transport.send(res).await.unwrap();
                }
            });
        }
    }

    async fn get_client(&mut self) -> Self::Client {
        IpcClient::new(&self.app_id, self.codec.clone())
    }
}
