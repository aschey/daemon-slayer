use futures::channel::oneshot;
use futures::future::Ready;
use futures::stream::{AbortHandle, Abortable};
use futures::{future, Future, Sink, SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use std::collections::HashMap;
use std::io;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tarpc::client::{self, Config, NewClient};
use tarpc::context::{self, Context};
use tarpc::serde::{Deserialize, Serialize};
use tarpc::serde_transport as transport;
use tarpc::serde_transport::Transport;
use tarpc::server::incoming::Incoming;
use tarpc::server::{BaseChannel, Channel, Serve};
use tarpc::tokio_serde::formats::{Bincode, Json};
use tarpc::tokio_serde::{Deserializer, Serializer};
use tarpc::tokio_util::codec::length_delimited::LengthDelimitedCodec;
use tarpc::tokio_util::codec::Decoder;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{Mutex, RwLock};
use tokio_serde::formats::{Cbor, MessagePack};

pub use tarpc;
pub use tarpc::{transport::channel::UnboundedChannel, ClientMessage, Response};

mod ipc_client;
pub use ipc_client::*;

mod ipc_request_handler;
pub use ipc_request_handler::*;

mod ipc_client_stream;
mod ipc_server;
pub use ipc_server::*;
pub mod pubsub;
pub mod rpc;
mod two_way;

#[derive(Clone, Debug)]
pub enum Codec {
    Bincode,
    Json,
    MessagePack,
    Cbor,
}

pub(crate) struct CodecWrapper<Item, SinkItem>
where
    SinkItem: Serialize + Unpin,
    Item: for<'de> Deserialize<'de> + Unpin,
{
    codec: Codec,
    phantom: PhantomData<(Item, SinkItem)>,
}

impl<Item, SinkItem> CodecWrapper<Item, SinkItem>
where
    SinkItem: Serialize + Unpin,
    Item: for<'de> Deserialize<'de> + Unpin,
{
    pub(crate) fn new(codec: Codec) -> Self {
        Self {
            codec,
            phantom: Default::default(),
        }
    }
}

impl<Item, SinkItem> Serializer<SinkItem> for CodecWrapper<Item, SinkItem>
where
    SinkItem: Serialize + Unpin,
    Item: for<'de> Deserialize<'de> + Unpin,
{
    type Error = io::Error;

    fn serialize(self: Pin<&mut Self>, item: &SinkItem) -> Result<bytes::Bytes, Self::Error> {
        match self.codec {
            Codec::Bincode => Pin::new(&mut Bincode::<Item, SinkItem>::default()).serialize(item),
            Codec::Json => Pin::new(&mut Json::<Item, SinkItem>::default())
                .serialize(item)
                .map_err(|e| e.into()),
            Codec::MessagePack => {
                Pin::new(&mut MessagePack::<Item, SinkItem>::default()).serialize(item)
            }
            Codec::Cbor => Pin::new(&mut Cbor::<Item, SinkItem>::default()).serialize(item),
        }
    }
}

impl<Item, SinkItem> Deserializer<Item> for CodecWrapper<Item, SinkItem>
where
    SinkItem: Serialize + Unpin,
    Item: for<'de> Deserialize<'de> + Unpin,
{
    type Error = io::Error;

    fn deserialize(self: Pin<&mut Self>, src: &bytes::BytesMut) -> Result<Item, Self::Error> {
        match self.codec {
            Codec::Bincode => Pin::new(&mut Bincode::<Item, SinkItem>::default()).deserialize(src),
            Codec::Json => Pin::new(&mut Json::<Item, SinkItem>::default())
                .deserialize(src)
                .map_err(|e| e.into()),
            Codec::MessagePack => {
                Pin::new(&mut MessagePack::<Item, SinkItem>::default()).deserialize(src)
            }
            Codec::Cbor => Pin::new(&mut Cbor::<Item, SinkItem>::default()).deserialize(src),
        }
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
