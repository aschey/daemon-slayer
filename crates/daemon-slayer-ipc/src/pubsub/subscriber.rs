use std::{
    fmt::{Debug, Display},
    ops::Deref,
    pin::Pin,
    str::FromStr,
};

use bytes::{Bytes, BytesMut};
use daemon_slayer_core::server::{FutureExt, SubsystemHandle};
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

use super::service::SubscriberService;

#[derive(Clone, Debug)]
pub(crate) struct Subscriber<T, M>
where
    T: FromStr + Display + Clone + Send + 'static,
    <T as FromStr>::Err: Debug,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Unpin + 'static,
{
    id: u32,
    app_id: String,
    codec: Codec,
    topics: Vec<String>,
    sender: tokio::sync::mpsc::Sender<(T, M)>,
}

impl<T, M> SubscriberService for Subscriber<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    type TopicsFut = future::Ready<Vec<String>>;
    fn topics(self, _: Context) -> Self::TopicsFut {
        future::ready(self.topics)
    }

    type ReceiveFut = Pin<Box<dyn Future<Output = ()> + Send>>;
    fn receive(self, _: Context, topic: String, message: Bytes) -> Self::ReceiveFut {
        let mut_message = BytesMut::from(message.deref());
        let value = Pin::new(&mut CodecWrapper::<M, M>::new(self.codec))
            .deserialize(&mut_message)
            .unwrap();

        Box::pin(async move {
            self.sender
                .send((T::from_str(&topic).unwrap(), value))
                .await
                .unwrap();
        })
    }

    type IdFut = Ready<u32>;
    fn id(self, _: Context) -> Self::IdFut {
        future::ready(self.id)
    }
}

impl<T, M> Subscriber<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    pub async fn new(
        app_id: impl Into<String>,
        sender: tokio::sync::mpsc::Sender<(T, M)>,
        topics: Vec<T>,
        codec: Codec,
    ) -> Self {
        Self {
            id: rand::random::<u32>(),
            app_id: app_id.into(),
            topics: topics.iter().map(|t| t.to_string()).collect(),
            sender,
            codec,
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) {
        let bind_addr = get_socket_address(&self.app_id, "subscriber");
        let publisher = IpcClientStream::new(bind_addr);

        let publisher = build_transport(publisher, CodecWrapper::new(self.codec.clone()));

        let mut handler = BaseChannel::with_defaults(publisher).requests();

        // The first request is for the topics being subscribed to.
        match handler.next().await {
            Some(id) => id.unwrap().execute(self.clone().serve()).await,
            None => panic!("test"),
        };

        match handler.next().await {
            Some(init_topics) => init_topics.unwrap().execute(self.clone().serve()).await,
            None => panic!("test"),
        };
        handler
            .execute(self.serve())
            .cancel_on_shutdown(&subsys)
            .await;
    }
}
