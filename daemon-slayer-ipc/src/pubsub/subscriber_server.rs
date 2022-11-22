use std::{
    fmt::{Debug, Display},
    ops::Deref,
    pin::Pin,
    str::FromStr,
};

use bytes::{Bytes, BytesMut};
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

use super::{service::SubscriberService, SubscriberHandle};

#[derive(Clone, Debug)]
pub(crate) struct SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Send + 'static,
    <T as FromStr>::Err: Debug,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Unpin + 'static,
{
    id: u32,
    codec: Codec,
    topics: Vec<String>,
    sender: mpsc::Sender<(T, M)>,
}

impl<T, M> SubscriberService for SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    type TopicsFut = future::Ready<Vec<String>>;
    fn topics(self, _: Context) -> Self::TopicsFut {
        println!("topics handler {:?}", self.topics);
        future::ready(self.topics)
    }

    type ReceiveFut = Pin<Box<dyn Future<Output = ()> + Send>>;
    fn receive(self, _: Context, topic: String, message: Bytes) -> Self::ReceiveFut {
        println!("received message");
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

impl<T, M> SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    pub async fn connect(
        app_id: &str,
        topics: &[T],
        sender: mpsc::Sender<(T, M)>,
        codec: Codec,
    ) -> SubscriberHandle {
        let bind_addr = get_socket_address(app_id, "subscriber");
        let publisher = IpcClientStream::new(bind_addr);
        println!("abc");
        let publisher = build_transport(publisher, CodecWrapper::new(codec.clone()));

        let id = rand::random::<u32>();
        let mut handler = BaseChannel::with_defaults(publisher).requests();
        let topics = topics.iter().map(|t| t.to_string()).collect();
        let subscriber = SubscriberServer {
            id,
            sender,
            topics,
            codec,
        };
        println!("yoyo");
        // The first request is for the topics being subscribed to.
        match handler.next().await {
            Some(id) => id.unwrap().execute(subscriber.clone().serve()).await,
            None => panic!("test"),
        };

        match handler.next().await {
            Some(init_topics) => {
                init_topics
                    .unwrap()
                    .execute(subscriber.clone().serve())
                    .await
            }
            None => panic!("test"),
        };
        println!("lala");
        let (handler, abort_handle) = future::abortable(handler.execute(subscriber.serve()));
        tokio::spawn(async move {
            match handler.await {
                Ok(()) | Err(future::Aborted) => println!("subscriber shutdown."),
            }
        });
        SubscriberHandle(abort_handle)
    }
}
