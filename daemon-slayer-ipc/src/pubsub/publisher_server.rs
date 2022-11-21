use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex, RwLock},
};

use bytes::Bytes;
use futures::{channel::oneshot, future, Future, StreamExt};
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use tarpc::{
    client, context,
    server::{BaseChannel, Channel},
};

use crate::{build_transport, get_socket_address, Codec, CodecWrapper};

use super::{subscription::Subscription, PublisherService, SubscriberServiceClient};

#[derive(Clone, Debug)]
pub struct PublisherServer {
    app_id: String,
    clients: Arc<Mutex<HashMap<u32, Subscription>>>,
    subscriptions: Arc<RwLock<HashMap<String, HashMap<u32, SubscriberServiceClient>>>>,
    codec: Codec,
}

impl PublisherServer {
    pub fn new(id: &str, codec: Codec) -> Self {
        Self {
            app_id: id.to_owned(),
            clients: Default::default(),
            subscriptions: Default::default(),
            codec,
        }
    }

    pub async fn start(self) {
        let bind_addr = get_socket_address(&self.app_id, "publisher");
        let mut endpoint = Endpoint::new(bind_addr);
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        self.clone().start_subscription_manager().await;
        let codec = self.codec.clone();
        tokio::spawn(async move {
            let incoming = endpoint.incoming().unwrap();
            futures::pin_mut!(incoming);
            while let Some(Ok(publisher)) = incoming.next().await {
                BaseChannel::with_defaults(build_transport(
                    publisher,
                    CodecWrapper::new(codec.clone()),
                ))
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

                let transport = build_transport(conn, CodecWrapper::new(self.codec.clone()));
                let tarpc::client::NewClient {
                    client: subscriber,
                    dispatch,
                } = SubscriberServiceClient::new(client::Config::default(), transport);

                let (ready_tx, ready) = oneshot::channel();

                self.clone().start_subscriber_gc(dispatch, ready);

                // Populate the topics
                let id = self.initialize_subscription(subscriber).await;

                // Signal that initialization is done.
                ready_tx.send(id).unwrap();
            }
        });
    }

    async fn initialize_subscription(&mut self, subscriber: SubscriberServiceClient) -> u32 {
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
impl PublisherService for PublisherServer {
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
