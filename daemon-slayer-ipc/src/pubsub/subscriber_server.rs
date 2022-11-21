use std::{ops::Deref, pin::Pin};

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
use tokio_serde::Deserializer;

use crate::{build_transport, get_socket_address, Codec, CodecWrapper};

use super::{Subscriber, SubscriberHandle, SubscriberService};

#[derive(Clone, Debug)]
pub struct SubscriberServer<S>
where
    S: Subscriber,
{
    id: u32,
    codec: Codec,
    subscriber: S,
}

impl<S> SubscriberService for SubscriberServer<S>
where
    S: Subscriber,
{
    type TopicsFut = Pin<Box<dyn Future<Output = Vec<String>> + Send>>;
    fn topics(self, _: Context) -> Self::TopicsFut {
        Box::pin(async move {
            self.subscriber
                .topics()
                .await
                .into_iter()
                .map(|t| t.to_string())
                .collect()
        })
    }

    type ReceiveFut = Pin<Box<dyn Future<Output = ()> + Send>>;
    fn receive(self, _: Context, topic: String, message: Bytes) -> Self::ReceiveFut {
        let mut_message = BytesMut::from(message.deref());
        let value = Pin::new(&mut CodecWrapper::<S::Message, S::Message>::new(self.codec))
            .deserialize(&mut_message)
            .unwrap();

        Box::pin(async move { self.subscriber.on_event(topic, value).await })
    }

    type IdFut = Ready<u32>;
    fn id(self, _: Context) -> Self::IdFut {
        future::ready(self.id)
    }
}

impl<S> SubscriberServer<S>
where
    S: Subscriber,
{
    pub async fn connect(app_id: &str, subscriber: S, codec: Codec) -> SubscriberHandle {
        let bind_addr = get_socket_address(app_id, "subscriber");
        let publisher = Endpoint::connect(bind_addr)
            .await
            .expect("Failed to connect client.");

        let publisher = build_transport(publisher, CodecWrapper::new(codec.clone()));

        let id = rand::random::<u32>();
        let mut handler = BaseChannel::with_defaults(publisher).requests();
        let subscriber = SubscriberServer {
            id,
            subscriber,
            codec,
        };
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
        let (handler, abort_handle) = future::abortable(handler.execute(subscriber.serve()));
        tokio::spawn(async move {
            match handler.await {
                Ok(()) | Err(future::Aborted) => println!("subscriber shutdown."),
            }
        });
        SubscriberHandle(abort_handle)
    }
}
