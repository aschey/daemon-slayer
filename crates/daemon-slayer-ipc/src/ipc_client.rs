use crate::{get_socket_address, ipc_client_stream::IpcClientStream, Codec, CodecWrapper};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_util::codec::{self, LengthDelimitedCodec};

pub struct IpcClient<Req, Res>
where
    Req: Serialize + for<'de> Deserialize<'de> + Unpin + Send,
    Res: Serialize + for<'de> Deserialize<'de> + Unpin + Send,
{
    stream: tokio_serde::Framed<
        codec::Framed<IpcClientStream, LengthDelimitedCodec>,
        Res,
        Req,
        CodecWrapper<Res, Req>,
    >,
}

impl<Req, Res> IpcClient<Req, Res>
where
    Req: Serialize + for<'de> Deserialize<'de> + Unpin + Send,
    Res: Serialize + for<'de> Deserialize<'de> + Unpin + Send,
{
    pub fn new(app_id: impl AsRef<str>, codec: Codec) -> Self {
        let client = IpcClientStream::new(get_socket_address(app_id.as_ref(), ""));
        let length_delimited = tokio_util::codec::Framed::new(client, LengthDelimitedCodec::new());
        let stream =
            tokio_serde::Framed::new(length_delimited, CodecWrapper::<Res, Req>::new(codec));

        Self { stream }
    }

    pub async fn send(&mut self, req: Req) -> Res {
        self.stream.send(req).await.unwrap();
        self.stream.next().await.unwrap().unwrap()
    }
}
