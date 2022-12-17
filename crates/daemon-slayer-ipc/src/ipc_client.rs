use std::{marker::PhantomData, mem, pin::Pin};

use bytes::{Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use parity_tokio_ipc::Endpoint;
use serde::{Deserialize, Serialize};
use tarpc::serde_transport::Transport;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio_serde::{Deserializer, Serializer};

use crate::{
    build_transport, get_socket_address, ipc_client_stream::IpcClientStream, Codec, CodecWrapper,
};

pub struct IpcClient<Req, Res>
where
    Req: Serialize + for<'de> Deserialize<'de> + Unpin + Send,
    Res: Serialize + for<'de> Deserialize<'de> + Unpin + Send,
{
    transport: Transport<IpcClientStream, Res, Req, CodecWrapper<Res, Req>>,
}

impl<Req, Res> IpcClient<Req, Res>
where
    Req: Serialize + for<'de> Deserialize<'de> + Unpin + Send,
    Res: Serialize + for<'de> Deserialize<'de> + Unpin + Send,
{
    pub fn new(app_id: impl AsRef<str>, codec: Codec) -> Self {
        let client = IpcClientStream::new(get_socket_address(app_id.as_ref(), ""));
        let transport = build_transport(client, CodecWrapper::<Res, Req>::new(codec));
        Self { transport }
    }

    pub async fn send(&mut self, req: Req) -> Res {
        self.transport.send(req).await.unwrap();
        self.transport.next().await.unwrap().unwrap()
    }
}
