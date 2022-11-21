use std::{fmt::Display, marker::PhantomData, pin::Pin};

use parity_tokio_ipc::Endpoint;
use tarpc::{client, context};
use tokio_serde::Serializer;

use crate::{build_transport, get_socket_address, Codec, CodecWrapper};

use super::PublisherServiceClient;

pub struct Publisher<T, M>
where
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + 'static,
    T: Display,
{
    client: PublisherServiceClient,
    message_phantom: PhantomData<M>,
    codec: Codec,
    topic_phantom: PhantomData<T>,
}

impl<T, M> Publisher<T, M>
where
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Unpin + 'static,
    T: Display,
{
    pub(crate) fn from_client(client: PublisherServiceClient, codec: Codec) -> Self {
        Self {
            client,
            message_phantom: Default::default(),
            topic_phantom: Default::default(),
            codec,
        }
    }
    pub async fn publish(&mut self, topic: T, message: M) {
        let value = Pin::new(&mut CodecWrapper::<M, M>::new(self.codec.clone()))
            .serialize(&message)
            .unwrap();

        self.client
            .publish(context::current(), topic.to_string(), value)
            .await
            .unwrap();
    }
}

pub async fn get_publisher<T, M>(app_id: &str, codec: Codec) -> Publisher<T, M>
where
    T: Display,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Unpin + 'static,
{
    let bind_addr = get_socket_address(app_id, "publisher");
    let endpoint = Endpoint::connect(bind_addr).await.unwrap();
    let transport = build_transport(endpoint, CodecWrapper::new(codec.clone()));
    let client = PublisherServiceClient::new(client::Config::default(), transport).spawn();
    Publisher::from_client(client, codec)
}
