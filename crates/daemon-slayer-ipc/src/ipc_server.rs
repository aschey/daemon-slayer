use crate::{get_socket_address, Codec, CodecWrapper, IpcClient, IpcRequestHandler};
use daemon_slayer_core::{
    async_trait,
    server::{tokio_stream::StreamExt, BackgroundService, ServiceContext},
    BoxedError, FutureExt,
};
use futures::SinkExt;
use parity_tokio_ipc::Endpoint;
use tokio_util::codec::{self, LengthDelimitedCodec};

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
        let endpoint = Endpoint::new(get_socket_address(&app_id, ""));
        Self {
            app_id,
            endpoint,
            codec,
            handler,
        }
    }

    pub fn get_client(&self) -> IpcClient<H::Req, H::Res> {
        IpcClient::new(&self.app_id, self.codec.clone())
    }
}

#[async_trait]
impl<H> BackgroundService for IpcServer<H>
where
    H: IpcRequestHandler + 'static,
{
    fn name<'a>() -> &'a str {
        "ipc_server"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let incoming = self.endpoint.incoming().expect("failed to open new socket");
        futures::pin_mut!(incoming);

        while let Ok(Some(Ok(stream))) = incoming
            .next()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            let length_delimited = codec::Framed::new(stream, LengthDelimitedCodec::new());

            let mut stream =
                tokio_serde::Framed::new(length_delimited, CodecWrapper::new(self.codec.clone()));

            let mut handler = self.handler.clone();
            tokio::spawn(async move {
                while let Ok(Some(msg)) = stream.try_next().await {
                    let res = handler.handle_request(msg).await;
                    stream.send(res).await.unwrap();
                }
            });
        }
        Ok(())
    }
}
