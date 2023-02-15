use serde::{Deserialize, Serialize};
use std::io;
use std::marker::PhantomData;
use std::pin::Pin;
use tokio_serde::formats::{Bincode, Cbor, Json, MessagePack};
use tokio_serde::{Deserializer, Serializer};

mod ipc_client;
pub use ipc_client::*;

mod ipc_request_handler;
pub use ipc_request_handler::*;

mod ipc_client_stream;
mod ipc_server;
pub use ipc_server::*;

pub mod health_check;

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
    let suffix_full = if suffix.is_empty() {
        "".to_owned()
    } else {
        format!("_{suffix}")
    };

    #[cfg(unix)]
    let addr = format!("/tmp/{id}{suffix_full}.sock");
    #[cfg(windows)]
    let addr = format!("\\\\.\\pipe\\{id}{suffix_full}");
    addr
}
