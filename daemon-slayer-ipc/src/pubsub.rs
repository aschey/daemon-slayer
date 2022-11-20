use bytes::{Buf, Bytes, BytesMut};
use futures::{
    channel::oneshot,
    future::{self, AbortHandle, Ready},
    prelude::*,
};
use parity_tokio_ipc::{Connection, Endpoint, SecurityAttributes};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fmt::Display,
    hash::Hash,
    io,
    marker::PhantomData,
    net::SocketAddr,
    ops::Deref,
    pin::Pin,
    sync::{Arc, Mutex, RwLock},
};
use tarpc::{
    client, context, serde_transport,
    server::{self, Channel},
    tokio_serde::{
        formats::{Bincode, Json},
        Deserializer, Serializer,
    },
    tokio_util::codec::LengthDelimitedCodec,
    transport, ClientMessage, Response,
};
use tokio_serde::formats::{Cbor, MessagePack};

use crate::{build_transport, get_socket_address, Codec};

// #[async_trait::async_trait]
// pub trait PubSubPublisher {
//     type Message: serde::Serialize;
//     type Codec: Serializer<Self::Message>;

//     async fn publish(&self, topic: String, message: Self::Message);
//     fn make_codec(&self) -> Self::Codec;
// }

// struct PubSubConfig<T, M, C, F>
// where
//     T: Display,
//     M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + 'static,
//     C: Serializer<M> + Deserializer<M> + Unpin,
//     F: Fn() -> C,
// {
//     topic: T,
//     message_phantom: PhantomData<M>,
//     make_codec: F,
// }

// pub trait CodecFactory: Clone + Send + Sync + 'static {
//     type Encode: serde::Serialize + Clone + Send + 'static;
//     type Decode: for<'de> serde::Deserialize<'de> + Clone + Send + 'static;
//     type Codec: Serializer<Self::Encode> + Deserializer<Self::Decode> + Unpin + Send + 'static;

//     fn make_codec(&self) -> Self::Codec;
// }

#[async_trait::async_trait]
pub trait PubSubSubscriber: Clone + Send + Sync + 'static {
    type Topic: Display;
    type Message: serde::Serialize
        + for<'de> serde::Deserialize<'de>
        + Clone
        + Send
        + Unpin
        + 'static;

    async fn topics(&self) -> Vec<Self::Topic>;
    async fn on_event(&self, topic: String, message: Self::Message);
}

pub struct PubSubPublisher<T, M>
where
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + 'static,
    T: Display,
    // C: CodecFactory<Encode = M, Decode = M>,
    //<<C as CodecFactory>::Codec as tarpc::tokio_serde::Serializer<M>>::Error: std::fmt::Debug,
{
    client: PublisherClient,
    message_phantom: PhantomData<M>,
    codec: Codec, //codec_factory: C,
    topic_phantom: PhantomData<T>,
}

impl<T, M> PubSubPublisher<T, M>
where
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Unpin + 'static,
    T: Display,
    //C: CodecFactory<Encode = M, Decode = M>,
    //<<C as CodecFactory>::Codec as tarpc::tokio_serde::Serializer<M>>::Error: std::fmt::Debug,
{
    fn from_client(client: PublisherClient, codec: Codec) -> Self {
        Self {
            client,
            message_phantom: Default::default(),
            topic_phantom: Default::default(),
            codec,
        }
    }
    pub async fn publish(&mut self, topic: T, message: M) {
        // let value = Pin::new(&mut self.codec_factory.make_codec())
        //     .serialize(&message)
        //     .unwrap();
        let value = match self.codec {
            Codec::Bincode => Pin::new(&mut Bincode::<M, M>::default())
                .serialize(&message)
                .unwrap(),
            Codec::Json => Pin::new(&mut Json::<M, M>::default())
                .serialize(&message)
                .unwrap(),
            Codec::MessagePack => Pin::new(&mut MessagePack::<M, M>::default())
                .serialize(&message)
                .unwrap(),
            Codec::Cbor => Pin::new(&mut Cbor::<M, M>::default())
                .serialize(&message)
                .unwrap(),
        };

        self.client
            .publish(context::current(), topic.to_string(), value)
            .await
            .unwrap();
    }
}

#[tarpc::service]
pub trait Subscriber {
    async fn topics() -> Vec<String>;
    async fn receive(topic: String, message: Bytes);
    async fn id() -> u32;
}

#[tarpc::service]
pub trait Publisher {
    async fn publish(topic: String, message: Bytes);
}

#[derive(Clone, Debug)]
pub struct SubscriberServer<S>
where
    S: PubSubSubscriber,
    // C: CodecFactory,
    // <<C as CodecFactory>::Codec as tarpc::tokio_serde::Deserializer<
    //     <C as CodecFactory>::Decode,
    // >>::Error: std::fmt::Debug,
{
    id: u32,
    codec: Codec,
    //codec_factory: C,
    subscriber: S,
}

impl<S> Subscriber for SubscriberServer<S>
where
    S: PubSubSubscriber, //<Message = C::Decode>,
                         // C: CodecFactory,
                         // <<C as CodecFactory>::Codec as tarpc::tokio_serde::Deserializer<
                         //     <C as CodecFactory>::Decode,
                         // >>::Error: std::fmt::Debug,
{
    type TopicsFut = Pin<Box<dyn Future<Output = Vec<String>> + Send>>;
    fn topics(self, _: context::Context) -> Self::TopicsFut {
        // let topics = self.subscriber.clone().topics();
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
    fn receive(self, _: context::Context, topic: String, message: Bytes) -> Self::ReceiveFut {
        //let mut codec = self.codec_factory.make_codec();
        let mut_message = BytesMut::from(message.deref());
        // let value = Pin::new(&mut codec).deserialize(&mut_message).unwrap();
        let value = match self.codec {
            Codec::Bincode => Pin::new(&mut Bincode::<S::Message, S::Message>::default())
                .deserialize(&mut_message)
                .unwrap(),
            Codec::Json => Pin::new(&mut Json::<S::Message, S::Message>::default())
                .deserialize(&mut_message)
                .unwrap(),
            Codec::MessagePack => Pin::new(&mut MessagePack::<S::Message, S::Message>::default())
                .deserialize(&mut_message)
                .unwrap(),
            Codec::Cbor => Pin::new(&mut Cbor::<S::Message, S::Message>::default())
                .deserialize(&mut_message)
                .unwrap(),
        };
        Box::pin(async move { self.subscriber.on_event(topic, value).await })
    }

    type IdFut = Ready<u32>;
    fn id(self, _: context::Context) -> Self::IdFut {
        future::ready(self.id)
    }
}

pub struct SubscriberHandle(AbortHandle);

impl Drop for SubscriberHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl<S> SubscriberServer<S>
where
    S: PubSubSubscriber, //<Message = C::Decode>,
                         // C: CodecFactory,
                         // <<C as CodecFactory>::Codec as tarpc::tokio_serde::Deserializer<
                         //     <C as CodecFactory>::Decode,
                         // >>::Error: std::fmt::Debug,
{
    pub async fn connect(app_id: &str, subscriber: S, codec: Codec) -> SubscriberHandle {
        let bind_addr = get_socket_address(app_id, "subscriber");
        let publisher = Endpoint::connect(bind_addr)
            .await
            .expect("Failed to connect client.");

        let publisher = build_transport(publisher, Bincode::default());

        let id = rand::random::<u32>();
        let mut handler = server::BaseChannel::with_defaults(publisher).requests();
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

#[derive(Debug)]
pub struct Subscription {
    topics: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PublisherServer {
    app_id: String,
    clients: Arc<Mutex<HashMap<u32, Subscription>>>,
    subscriptions: Arc<RwLock<HashMap<String, HashMap<u32, SubscriberClient>>>>,
}

impl PublisherServer {
    pub fn new(id: &str) -> Self {
        Self {
            app_id: id.to_owned(),
            clients: Default::default(),
            subscriptions: Default::default(),
        }
    }

    pub async fn start(self) {
        let bind_addr = get_socket_address(&self.app_id, "publisher");
        let mut endpoint = Endpoint::new(bind_addr);
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        self.clone().start_subscription_manager().await;

        tokio::spawn(async move {
            let incoming = endpoint.incoming().unwrap();
            futures::pin_mut!(incoming);
            while let Some(Ok(publisher)) = incoming.next().await {
                let transport = build_transport(publisher, Bincode::default());

                server::BaseChannel::with_defaults(transport)
                    .execute(self.clone().serve())
                    .await
            }
        });
    }

    async fn start_subscription_manager(mut self) {
        let bind_addr = get_socket_address(&self.app_id, "subscriber");
        let mut endpoint = Endpoint::new(bind_addr);

        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        tokio::spawn(async move {
            let connecting_subscribers = endpoint.incoming().unwrap();
            futures::pin_mut!(connecting_subscribers);
            while let Some(Ok(conn)) = connecting_subscribers.next().await {
                //let subscriber_addr = conn.peer_addr().unwrap();

                let transport = build_transport(conn, Bincode::default());
                let tarpc::client::NewClient {
                    client: subscriber,
                    dispatch,
                } = SubscriberClient::new(client::Config::default(), transport);
                let (ready_tx, ready) = oneshot::channel();

                self.clone().start_subscriber_gc(dispatch, ready);

                // Populate the topics
                let id = self.initialize_subscription(subscriber).await;

                // Signal that initialization is done.
                ready_tx.send(id).unwrap();
            }
        });
    }

    async fn initialize_subscription(&mut self, subscriber: SubscriberClient) -> u32 {
        let id = subscriber.id(context::current()).await.unwrap();
        // Populate the topics
        if let Ok(topics) = subscriber.topics(context::current()).await {
            self.clients.lock().unwrap().insert(
                id,
                Subscription {
                    topics: topics.clone(),
                },
            );

            println!("subscribed to new topics");
            let mut subscriptions = self.subscriptions.write().unwrap();
            for topic in topics {
                subscriptions
                    .entry(topic)
                    .or_insert_with(HashMap::new)
                    .insert(id, subscriber.clone());
            }
        }

        id
    }

    fn start_subscriber_gc<E: Error>(
        self,
        client_dispatch: impl Future<Output = Result<(), E>> + Send + 'static,
        subscriber_ready: oneshot::Receiver<u32>,
    ) {
        tokio::spawn(async move {
            if let Err(e) = client_dispatch.await {
                println!("subscriber connection broken");
            }
            // Don't clean up the subscriber until initialization is done.
            let id = subscriber_ready.await.unwrap();
            if let Some(subscription) = self.clients.lock().unwrap().remove(&id) {
                println!("{:?}", subscription.topics);
                let mut subscriptions = self.subscriptions.write().unwrap();
                for topic in subscription.topics {
                    let subscribers = subscriptions.get_mut(&topic).unwrap();
                    subscribers.remove(&id);
                    if subscribers.is_empty() {
                        subscriptions.remove(&topic);
                    }
                }
            }
        });
    }
}

#[tarpc::server]
impl Publisher for PublisherServer {
    async fn publish(self, _: context::Context, topic: String, message: Bytes) {
        println!("received message to publish.");
        let mut subscribers = match self.subscriptions.read().unwrap().get(&topic) {
            None => return,
            Some(subscriptions) => subscriptions.clone(),
        };
        let mut publications = Vec::new();
        for client in subscribers.values_mut() {
            publications.push(client.receive(context::current(), topic.clone(), message.clone()));
        }
        // Ignore failing subscribers. In a real pubsub, you'd want to continually retry until
        // subscribers ack. Of course, a lot would be different in a real pubsub :)
        for response in future::join_all(publications).await {
            if let Err(e) = response {
                println!("failed to broadcast to subscriber: {}", e);
            }
        }
    }
}

pub async fn get_publisher<T, M>(app_id: &str, codec: Codec) -> PubSubPublisher<T, M>
where
    T: Display,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Unpin + 'static,
    // C1: CodecFactory<
    //     Encode = tarpc::ClientMessage<PublisherRequest>,
    //     Decode = tarpc::Response<PublisherResponse>,
    // >,
    // C2: CodecFactory<Encode=M, Decode=M>,
    // <<C2 as CodecFactory>::Codec as tarpc::tokio_serde::Serializer<
    //     <C2 as CodecFactory>::Encode,
    // >>::Error: std::fmt::Debug,
    // std::io::Error: From<<<C1 as CodecFactory>::Codec as tarpc::tokio_serde::Serializer<ClientMessage<PublisherRequest>>>::Error>,
    // std::io::Error: From<<<C1 as CodecFactory>::Codec as tarpc::tokio_serde::Deserializer<Response<PublisherResponse>>>::Error>,
    // for<'de> <C2 as CodecFactory>::Encode: serde::Deserialize<'de>,
{
    let bind_addr = get_socket_address(app_id, "publisher");
    let endpoint = Endpoint::connect(bind_addr).await.unwrap();
    let transport = build_transport(endpoint, Bincode::default());
    let client = PublisherClient::new(client::Config::default(), transport).spawn();
    PubSubPublisher::from_client(client, codec)
}
