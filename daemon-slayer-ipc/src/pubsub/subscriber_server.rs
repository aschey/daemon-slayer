use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
    str::FromStr,
};

use bytes::{Bytes, BytesMut};
use daemon_slayer_core::server::{BackgroundService, FutureExt, ServiceContext, SubsystemHandle};
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

use super::{service::SubscriberService, subscriber::Subscriber, SubscriberClient};

#[derive(Debug)]
pub struct SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Unpin + 'static,
{
    app_id: String,
    codec: Codec,
    subscriber_tx: tokio::sync::mpsc::Sender<(Vec<T>, tokio::sync::mpsc::Sender<(T, M)>)>,
    subscriber_rx: tokio::sync::mpsc::Receiver<(Vec<T>, tokio::sync::mpsc::Sender<(T, M)>)>,
}

impl<T, M> SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    pub fn new(app_id: impl Into<String>, codec: Codec) -> Self {
        let (subscriber_tx, subscriber_rx) = tokio::sync::mpsc::channel(32);
        Self {
            app_id: app_id.into(),
            codec,
            subscriber_tx,
            subscriber_rx,
        }
    }
}

#[async_trait::async_trait]
impl<T, M> BackgroundService for SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    type Client = SubscriberClient<T, M>;

    fn name<'a>() -> &'a str {
        "subscriber_server"
    }

    async fn run(mut self, context: ServiceContext) {
        let mut subscriber_handles = vec![];
        while let Ok(Some((topics, tx))) = self
            .subscriber_rx
            .recv()
            .cancel_on_shutdown(&context.get_subsystem_handle())
            .await
        {
            let subscriber = Subscriber::new(&self.app_id, tx, topics, self.codec.clone()).await;
            let subsys = context.get_subsystem_handle();
            subscriber_handles.push(tokio::spawn(async move {
                subscriber.run(subsys).await;
            }));
        }
    }

    async fn get_client(&mut self) -> Self::Client {
        SubscriberClient::new(self.subscriber_tx.clone())
    }
}
