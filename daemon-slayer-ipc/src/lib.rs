use std::io;
use std::time::Duration;

use futures::future::Ready;
use futures::stream::{AbortHandle, Abortable};
use futures::{future, Sink, SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use tarpc::client::{self, Config, NewClient};
use tarpc::context::{self, Context};
use tarpc::serde::{Deserialize, Serialize};
use tarpc::serde_transport::Transport;
use tarpc::server::incoming::Incoming;
use tarpc::server::{BaseChannel, Channel, Serve};
use tarpc::tokio_serde::formats::Bincode;
use tarpc::tokio_serde::{Deserializer, Serializer};
use tarpc::tokio_util::codec::length_delimited::LengthDelimitedCodec;
use tarpc::transport::channel::UnboundedChannel;
use tarpc::{serde_transport as transport, ClientMessage, Response};
use tokio::io::{AsyncRead, AsyncWrite};

pub struct RpcService {}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum TwoWayMessage<Req, Resp> {
    Request(tarpc::ClientMessage<Req>),
    Response(tarpc::Response<Resp>),
}

impl RpcService {
    pub fn spawn_server<Req, Resp, Service, MakeService, Codec, MakeCodec>(
        id: String,
        make_service: MakeService,
        make_codec: MakeCodec,
    ) where
        Resp: Serialize + for<'de> Deserialize<'de> + Send + 'static,
        Req: Serialize + for<'de> Deserialize<'de> + Send + 'static,
        Service: Serve<Req, Resp = Resp> + Send + Clone + 'static,
        Service::Fut: Send,
        MakeCodec: Fn() -> Codec + Send + Sync + 'static,
        MakeService: Fn(UnboundedChannel<Response<Resp>, ClientMessage<Req>>) -> Service
            + Send
            + Sync
            + 'static,
        Codec: Serializer<TwoWayMessage<Req, Resp>>
            + Deserializer<TwoWayMessage<Req, Resp>>
            + Unpin
            + Default
            + Send
            + 'static,
        std::io::Error: std::convert::From<
            <Codec as tarpc::tokio_serde::Deserializer<TwoWayMessage<Req, Resp>>>::Error,
        >,
        std::io::Error: std::convert::From<
            <Codec as tarpc::tokio_serde::Serializer<TwoWayMessage<Req, Resp>>>::Error,
        >,
    {
        #[cfg(unix)]
        let bind_addr = format!("/tmp/{id}_rpc.sock");
        #[cfg(windows)]
        let bind_addr = format!("\\\\.\\pipe\\{id}_rpc");

        let mut endpoint = Endpoint::new(bind_addr);

        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());
        let mut codec_builder = LengthDelimitedCodec::builder();
        let incoming = endpoint.incoming().expect("failed to open new socket");
        tokio::spawn(async move {
            incoming
                .filter_map(|r| future::ready(r.ok()))
                .map(|stream| {
                    let framed = codec_builder
                        .max_frame_length(usize::MAX)
                        .new_framed(stream);

                    let transport = transport::new(framed, make_codec());

                    let (server_chan, client_chan) = Self::spawn_twoway(transport);
                    (BaseChannel::with_defaults(server_chan), client_chan)
                })
                .map(|(base_chan, client_chan)| base_chan.execute(make_service(client_chan)))
                .buffer_unordered(10)
                .for_each(|_| async {})
                .await;
        });
    }

    pub async fn get_client<Req, Resp, Service, MakeService, Codec, MakeCodec, MakeClient, Client>(
        id: String,
        make_service: MakeService,
        make_client: MakeClient,
        make_codec: MakeCodec,
    ) -> Client
    where
        Resp: Serialize + for<'de> Deserialize<'de> + Send + 'static,
        Req: Serialize + for<'de> Deserialize<'de> + Send + 'static,
        Service: Serve<Req, Resp = Resp> + Send + Clone + 'static,
        Service::Fut: Send,
        Client: Clone + Send + 'static,
        MakeCodec: Fn() -> Codec + Send + Sync + 'static,
        MakeService: Fn(Client) -> Service + Send + Sync + 'static,
        MakeClient: FnOnce(UnboundedChannel<Response<Resp>, ClientMessage<Req>>) -> Client
            + Send
            + Sync
            + 'static,
        Codec: Serializer<TwoWayMessage<Req, Resp>>
            + Deserializer<TwoWayMessage<Req, Resp>>
            + Unpin
            + Default
            + Send
            + 'static,
        std::io::Error: std::convert::From<
            <Codec as tarpc::tokio_serde::Deserializer<TwoWayMessage<Req, Resp>>>::Error,
        >,
        std::io::Error: std::convert::From<
            <Codec as tarpc::tokio_serde::Serializer<TwoWayMessage<Req, Resp>>>::Error,
        >,
    {
        #[cfg(unix)]
        let bind_addr = format!("/tmp/{id}_rpc.sock");
        #[cfg(windows)]
        let bind_addr = format!("\\\\.\\pipe\\{id}_rpc");
        let conn = Endpoint::connect(bind_addr.to_string())
            .await
            .expect("Failed to connect client.");
        let mut codec_builder = LengthDelimitedCodec::builder();
        let framed = codec_builder.max_frame_length(usize::MAX).new_framed(conn);

        let transport = transport::new(framed, make_codec());
        let (server_chan, client_chan) = Self::spawn_twoway(transport);
        let peer = make_client(client_chan);
        let peer_ = peer.clone();
        tokio::spawn(async move {
            let service = make_service(peer_);
            BaseChannel::with_defaults(server_chan)
                .execute(service)
                .await;
        });
        peer
    }

    fn spawn_twoway<Req1, Resp1, Req2, Resp2, T>(
        transport: T,
    ) -> (
        UnboundedChannel<tarpc::ClientMessage<Req1>, tarpc::Response<Resp1>>,
        UnboundedChannel<tarpc::Response<Resp2>, tarpc::ClientMessage<Req2>>,
    )
    where
        T: Stream<Item = Result<TwoWayMessage<Req1, Resp2>, io::Error>>,
        T: Sink<TwoWayMessage<Req2, Resp1>, Error = io::Error>,
        T: Unpin + Send + 'static,
        Req1: Send + 'static,
        Resp1: Send + 'static,
        Req2: Send + 'static,
        Resp2: Send + 'static,
    {
        let (server, server_ret) = tarpc::transport::channel::unbounded();
        let (client, client_ret) = tarpc::transport::channel::unbounded();
        let (mut server_sink, server_stream) = server.split();
        let (mut client_sink, client_stream) = client.split();
        let (transport_sink, mut transport_stream) = transport.split();

        let (abort_handle, abort_registration) = AbortHandle::new_pair();

        // Task for inbound message handling
        tokio::spawn(async move {
            while let Some(msg) = transport_stream.next().await {
                match msg.unwrap() {
                    TwoWayMessage::Request(req) => server_sink.send(req).await.unwrap(),
                    TwoWayMessage::Response(resp) => client_sink.send(resp).await.unwrap(),
                }
            }

            abort_handle.abort();
        });

        let abortable_sink_channel = Abortable::new(
            futures::stream::select(
                server_stream.map_ok(TwoWayMessage::Response),
                client_stream.map_ok(TwoWayMessage::Request),
            )
            .map_err(|e| e.to_string()),
            abort_registration,
        );

        // Task for outbound message handling
        tokio::spawn(
            abortable_sink_channel
                .forward(transport_sink.sink_map_err(|e| e.to_string()))
                .inspect_ok(|_| println!("transport_sink done"))
                .inspect_err(|e| println!("Error in outbound multiplexing: {}", e)),
        );

        (server_ret, client_ret)
    }
}
