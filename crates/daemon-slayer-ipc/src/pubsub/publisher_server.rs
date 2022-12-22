use crate::{build_transport, get_socket_address, Codec, CodecWrapper};
use bytes::Bytes;
use daemon_slayer_core::{
    server::{BackgroundService, ServiceContext},
    BoxedError, FutureExt,
};
use futures::{channel::oneshot, future, Future, StreamExt};
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
    marker::PhantomData,
    str::FromStr,
    sync::{Arc, Mutex, RwLock},
};
use tarpc::{
    client, context,
    server::{BaseChannel, Channel},
};

use super::{
    get_publisher,
    service::{PublisherService, SubscriberServiceClient},
    subscription::Subscription,
    Publisher,
};

#[derive(Clone, Debug)]
pub struct PublisherServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    app_id: String,
    clients: Arc<Mutex<HashMap<u32, Subscription>>>,
    subscriptions: Arc<RwLock<HashMap<String, HashMap<u32, SubscriberServiceClient>>>>,
    codec: Codec,
    phantom: PhantomData<(T, M)>,
}

impl<T, M> PublisherServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    pub fn new(id: &str, codec: Codec) -> Self {
        Self {
            app_id: id.to_owned(),
            clients: Default::default(),
            subscriptions: Default::default(),
            codec,
            phantom: Default::default(),
        }
    }

    async fn start_subscription_manager(mut self) {
        let bind_addr = get_socket_address(&self.app_id, "subscriber");
        let mut endpoint = Endpoint::new(bind_addr);

        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        tokio::spawn(async move {
            let connecting_subscribers = endpoint.incoming().unwrap();
            futures::pin_mut!(connecting_subscribers);
            while let Some(Ok(conn)) = connecting_subscribers.next().await {
                let transport = build_transport(conn, CodecWrapper::new(self.codec.clone()));
                let tarpc::client::NewClient {
                    client: subscriber,
                    dispatch,
                } = SubscriberServiceClient::new(client::Config::default(), transport);

                let (ready_tx, ready) = oneshot::channel();

                self.clone().start_subscriber_gc(dispatch, ready);

                let id = self.initialize_subscription(subscriber).await;

                ready_tx.send(id).unwrap();
            }
        });
    }

    async fn initialize_subscription(&mut self, subscriber: SubscriberServiceClient) -> u32 {
        println!("initialize sub");
        let id = subscriber.id(context::current()).await.unwrap();

        match subscriber.topics(context::current()).await {
            Ok(topics) => {
                println!("topics {topics:?}");
                self.clients.lock().unwrap().insert(
                    id,
                    Subscription {
                        topics: topics.clone(),
                    },
                );

                let mut subscriptions = self.subscriptions.write().unwrap();
                for topic in topics {
                    subscriptions
                        .entry(topic)
                        .or_insert_with(HashMap::new)
                        .insert(id, subscriber.clone());
                }
            }
            Err(e) => {
                println!("topics err {e:?}");
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
                println!("err {e}");
            }
            let id = subscriber_ready.await.unwrap();
            println!("subscriber ready");
            if let Some(subscription) = self.clients.lock().unwrap().remove(&id) {
                let mut subscriptions = self.subscriptions.write().unwrap();
                for topic in subscription.topics {
                    println!("removing");
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
impl<T, M> PublisherService for PublisherServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    async fn publish(self, _: context::Context, topic: String, message: Bytes) {
        let mut subscribers = match self.subscriptions.read().unwrap().get(&topic) {
            None => return,
            Some(subscriptions) => subscriptions.clone(),
        };
        let mut publications = Vec::new();
        for client in subscribers.values_mut() {
            println!("sending to subscriber");
            publications.push(client.receive(context::current(), topic.clone(), message.clone()));
        }

        for response in future::join_all(publications).await {
            if let Err(e) = response {
                println!("Err {e:?}");
            }
        }
    }
}

#[async_trait::async_trait]
impl<T, M> BackgroundService for PublisherServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    type Client = Publisher<T, M>;

    fn name<'a>() -> &'a str {
        "pubsub_server"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let bind_addr = get_socket_address(&self.app_id, "publisher");
        let mut endpoint = Endpoint::new(bind_addr);
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        let new = self.clone();
        new.start_subscription_manager().await;
        let codec = self.codec.clone();

        let incoming = endpoint.incoming().unwrap();
        futures::pin_mut!(incoming);
        while let Ok(Some(Ok(publisher))) = incoming
            .next()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            let new = self.clone();
            BaseChannel::with_defaults(build_transport(publisher, CodecWrapper::new(codec.clone())))
                .execute(new.serve())
                .await
        }

        Ok(())
    }

    async fn get_client(&mut self) -> Self::Client {
        get_publisher::<T, M>(self.app_id.as_ref(), self.codec.clone()).await
    }
}
