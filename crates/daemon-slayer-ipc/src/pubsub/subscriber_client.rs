use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
    str::FromStr,
};

use bytes::{Bytes, BytesMut};
use daemon_slayer_core::server::{BackgroundService, FutureExt, SubsystemHandle};
use futures::{
    future::{self, Ready},
    Future, StreamExt,
};
use parity_tokio_ipc::Endpoint;
use tarpc::{
    context::Context,
    server::{BaseChannel, Channel},
};
use tokio::sync::{broadcast, mpsc};
use tokio_serde::Deserializer;

use crate::{
    build_transport, get_socket_address, ipc_client_stream::IpcClientStream, Codec, CodecWrapper,
};

use super::{service::SubscriberService, subscriber::Subscriber};

pub struct SubscriberClient<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    subscriber_tx: tokio::sync::mpsc::Sender<(Vec<T>, tokio::sync::mpsc::Sender<(T, M)>)>,
}

impl<T, M> SubscriberClient<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    pub(crate) fn new(
        subscriber_tx: tokio::sync::mpsc::Sender<(Vec<T>, tokio::sync::mpsc::Sender<(T, M)>)>,
    ) -> Self {
        Self { subscriber_tx }
    }

    pub async fn subscribe(
        &mut self,
        topics: impl Into<Vec<T>>,
    ) -> tokio::sync::mpsc::Receiver<(T, M)> {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        self.subscriber_tx.send((topics.into(), tx)).await;
        rx
    }
}
