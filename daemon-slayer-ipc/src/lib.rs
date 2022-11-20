use futures::channel::oneshot;
use futures::future::Ready;
use futures::stream::{AbortHandle, Abortable};
use futures::{future, Future, Sink, SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use std::collections::HashMap;
use std::io;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
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
use tokio::sync::{Mutex, RwLock};

mod pubsub;
pub use pubsub::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum TwoWayMessage<Req, Resp> {
    Request(tarpc::ClientMessage<Req>),
    Response(tarpc::Response<Resp>),
}

pub trait ServiceFactory: Clone + Send + Sync + 'static
where
    std::io::Error: std::convert::From<
        <Self::Codec as tarpc::tokio_serde::Deserializer<TwoWayMessage<Self::Req, Self::Resp>>>::Error,
    >,
    std::io::Error: std::convert::From<
        <Self::Codec as tarpc::tokio_serde::Serializer<TwoWayMessage<Self::Req, Self::Resp>>>::Error,
    >,
{
    type Resp: Serialize + for<'de> Deserialize<'de> + Send + Unpin + 'static;
    type Req: Serialize + for<'de> Deserialize<'de> + Send + Unpin + 'static;
    type Client: Clone + Send + 'static;

    type Service: Serve<Self::Req, Resp = Self::Resp> + Send + Clone + 'static;

    type Codec: Serializer<TwoWayMessage<Self::Req, Self::Resp>>
        + Deserializer<TwoWayMessage<Self::Req, Self::Resp>>
        + Unpin
        + Default
        + Send
        + 'static;
    fn make_service(&self, client: Self::Client) -> Self::Service;
    fn make_client(
        &self,
        chan: UnboundedChannel<Response<Self::Resp>, ClientMessage<Self::Req>>,
    ) -> Self::Client;
    fn make_codec(&self) -> Self::Codec;
}

pub struct RpcService<F: ServiceFactory>
where
    <<F as ServiceFactory>::Service as tarpc::server::Serve<<F as ServiceFactory>::Req>>::Fut: Send,
    std::io::Error: std::convert::From<
        <F::Codec as tarpc::tokio_serde::Deserializer<TwoWayMessage<F::Req, F::Resp>>>::Error,
    >,
    std::io::Error: std::convert::From<
        <F::Codec as tarpc::tokio_serde::Serializer<TwoWayMessage<F::Req, F::Resp>>>::Error,
    >,
{
    bind_addr: String,
    service_factory: F,
}

impl<F: ServiceFactory> RpcService<F>
where
    <<F as ServiceFactory>::Service as tarpc::server::Serve<<F as ServiceFactory>::Req>>::Fut: Send,
    std::io::Error: std::convert::From<
        <F::Codec as tarpc::tokio_serde::Deserializer<TwoWayMessage<F::Req, F::Resp>>>::Error,
    >,
    std::io::Error: std::convert::From<
        <F::Codec as tarpc::tokio_serde::Serializer<TwoWayMessage<F::Req, F::Resp>>>::Error,
    >,
{
    pub fn new(id: &str, service_factory: F) -> Self {
        Self {
            bind_addr: get_socket_address(id, "rpc"),
            service_factory,
        }
    }
    pub fn spawn_server(&self) {
        let mut endpoint = Endpoint::new(self.bind_addr.clone());
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());
        let mut codec_builder = LengthDelimitedCodec::builder();
        let incoming = endpoint.incoming().expect("failed to open new socket");
        let service_factory = self.service_factory.clone();
        tokio::spawn(async move {
            incoming
                .filter_map(|r| future::ready(r.ok()))
                .map(|stream| {
                    let transport = build_transport(stream, service_factory.make_codec());

                    let (server_chan, client_chan) = Self::spawn_twoway(transport);
                    let peer = service_factory.make_client(client_chan);
                    (BaseChannel::with_defaults(server_chan), peer)
                })
                .map(|(base_chan, peer)| base_chan.execute(service_factory.make_service(peer)))
                .buffer_unordered(10)
                .for_each(|_| async {})
                .await;
        });
    }

    pub async fn get_client(&self) -> F::Client {
        let conn = Endpoint::connect(self.bind_addr.clone())
            .await
            .expect("Failed to connect client.");

        let transport = build_transport(conn, self.service_factory.make_codec());
        let (server_chan, client_chan) = Self::spawn_twoway(transport);
        let peer = self.service_factory.make_client(client_chan);
        let peer_ = peer.clone();
        let service_factory = self.service_factory.clone();
        tokio::spawn(async move {
            let service = service_factory.make_service(peer_);
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

pub(crate) fn get_socket_address(id: &str, suffix: &str) -> String {
    #[cfg(unix)]
    let addr = format!("/tmp/{}_{}.sock", id, suffix);
    #[cfg(windows)]
    let addr = format!("\\\\.\\pipe\\{}_{}", id, suffix);
    addr
}

pub(crate) fn build_transport<S, Item, SinkItem, Codec>(
    stream: S,
    codec: Codec,
) -> Transport<S, Item, SinkItem, Codec>
where
    S: AsyncRead + AsyncWrite,
    Item: for<'de> Deserialize<'de>,
    SinkItem: Serialize,
    Codec: Serializer<SinkItem> + Deserializer<Item>,
{
    let mut codec_builder = LengthDelimitedCodec::builder();
    let framed = codec_builder
        .max_frame_length(usize::MAX)
        .new_framed(stream);
    transport::new(framed, codec)
}
