use std::{
    fmt::{Debug, Display},
    str::FromStr,
};
use tokio::sync::mpsc;

pub struct SubscriberClient<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    subscriber_tx: mpsc::Sender<(Vec<T>, mpsc::Sender<(T, M)>)>,
}

impl<T, M> SubscriberClient<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    pub(crate) fn new(subscriber_tx: mpsc::Sender<(Vec<T>, mpsc::Sender<(T, M)>)>) -> Self {
        Self { subscriber_tx }
    }

    pub async fn subscribe(&mut self, topics: impl Into<Vec<T>>) -> mpsc::Receiver<(T, M)> {
        let (tx, rx) = mpsc::channel(32);
        self.subscriber_tx.send((topics.into(), tx)).await;
        rx
    }
}
