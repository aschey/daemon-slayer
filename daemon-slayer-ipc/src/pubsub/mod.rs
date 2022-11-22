mod subscription;

mod subscriber_server;
use std::{fmt::Display, marker::PhantomData, str::FromStr};

mod publisher_server;
pub use publisher_server::*;

mod publisher;
pub use publisher::*;

mod subscriber_handle;
pub use subscriber_handle::*;
use tokio::sync::mpsc;

use crate::Codec;
use std::fmt::Debug;

use self::subscriber_server::SubscriberServer;
mod service;

pub struct PubSub<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    phantom: PhantomData<(T, M)>,
}

impl<T, M> PubSub<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Debug + Send + Unpin + 'static,
{
    pub async fn get_publisher(
        app_id: impl AsRef<str>,
        codec: Codec,
    ) -> (PublisherServer, Publisher<T, M>) {
        (
            PublisherServer::new(app_id.as_ref(), codec.clone()),
            get_publisher::<T, M>(app_id.as_ref(), codec).await,
        )
    }

    pub async fn subscribe(
        app_id: impl AsRef<str>,
        topics: &[T],
        codec: Codec,
    ) -> (SubscriberHandle, mpsc::Receiver<(T, M)>) {
        let (tx, rx) = mpsc::channel(32);
        let subscriber =
            SubscriberServer::<T, M>::connect(app_id.as_ref(), topics, tx, codec.clone()).await;
        (subscriber, rx)
    }
}
