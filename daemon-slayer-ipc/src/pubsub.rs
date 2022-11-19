use futures::{
    channel::oneshot,
    future::{self, AbortHandle},
    prelude::*,
};
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use std::{
    collections::HashMap,
    env,
    error::Error,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};

use tarpc::{
    client, context, serde_transport,
    server::{self, Channel},
    tokio_serde::formats::{Bincode, Json},
    tokio_util::codec::LengthDelimitedCodec,
    transport,
};

#[tarpc::service]
pub trait Subscriber {
    async fn topics() -> Vec<String>;
    async fn receive(topic: String, message: String);
    async fn id() -> u32;
}
#[tarpc::service]
pub trait Publisher {
    async fn publish(topic: String, message: String);
}
#[derive(Clone, Debug)]
pub struct SubscriberServer {
    id: u32,
    topics: Vec<String>,
}

#[tarpc::server]
impl Subscriber for SubscriberServer {
    async fn topics(self, _: context::Context) -> Vec<String> {
        self.topics.clone()
    }

    async fn receive(self, _: context::Context, topic: String, message: String) {
        println!("ReceivedMessage");
    }

    async fn id(self, _: context::Context) -> u32 {
        self.id
    }
}

pub struct SubscriberHandle(AbortHandle);

impl Drop for SubscriberHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl SubscriberServer {
    pub async fn connect(topics: Vec<String>) -> SubscriberHandle {
        let bind_addr = format!("\\\\.\\pipe\\test_pubsub_subscriber");
        let publisher = Endpoint::connect(bind_addr)
            .await
            .expect("Failed to connect client.");

        let mut codec_builder = LengthDelimitedCodec::builder();
        let framed = codec_builder
            .max_frame_length(usize::MAX)
            .new_framed(publisher);

        let publisher = serde_transport::new(framed, Bincode::default());

        let id = rand::random::<u32>();
        let mut handler = server::BaseChannel::with_defaults(publisher).requests();
        let subscriber = SubscriberServer { id, topics };
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
    pub clients: Arc<Mutex<HashMap<u32, Subscription>>>,
    pub subscriptions: Arc<RwLock<HashMap<String, HashMap<u32, SubscriberClient>>>>,
}

// struct PublisherAddrs {
//     publisher: SocketAddr,
//     subscriptions: SocketAddr,
// }

impl PublisherServer {
    pub async fn start(self) {
        let bind_addr = format!("\\\\.\\pipe\\test_pubsub_publisher");
        let mut endpoint = Endpoint::new(bind_addr);
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        // let mut connecting_publishers = tcp::listen("localhost:0", Json::default).await?;

        self.clone().start_subscription_manager().await;
        // let publisher_addrs = PublisherAddrs {
        //     publisher: connecting_publishers.local_addr(),
        //     subscriptions: self.clone().start_subscription_manager().await?,
        // };

        //info!(publisher_addr = %publisher_addrs.publisher, "listening for publishers.",);
        let mut codec_builder = LengthDelimitedCodec::builder();
        tokio::spawn(async move {
            let incoming = endpoint.incoming().unwrap();
            futures::pin_mut!(incoming);
            while let Some(Ok(publisher)) = incoming.next().await {
                let framed = codec_builder
                    .max_frame_length(usize::MAX)
                    .new_framed(publisher);

                let transport = serde_transport::new(framed, Bincode::default());

                server::BaseChannel::with_defaults(transport)
                    .execute(self.clone().serve())
                    .await
            }
        });
    }

    async fn start_subscription_manager(mut self) {
        let bind_addr = format!("\\\\.\\pipe\\test_pubsub_subscriber");
        let mut endpoint = Endpoint::new(bind_addr);
        // let mut connecting_subscribers = tcp::listen("localhost:0", Json::default)
        //     .await?
        //     .filter_map(|r| future::ready(r.ok()));
        // let new_subscriber_addr = connecting_subscribers.get_ref().local_addr();
        // info!(?new_subscriber_addr, "listening for subscribers.");
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        let mut codec_builder = LengthDelimitedCodec::builder();
        tokio::spawn(async move {
            let connecting_subscribers = endpoint.incoming().unwrap();
            futures::pin_mut!(connecting_subscribers);
            while let Some(Ok(conn)) = connecting_subscribers.next().await {
                //let subscriber_addr = conn.peer_addr().unwrap();
                let conn = codec_builder.max_frame_length(usize::MAX).new_framed(conn);

                let transport = serde_transport::new(conn, Bincode::default());
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
    async fn publish(self, _: context::Context, topic: String, message: String) {
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
