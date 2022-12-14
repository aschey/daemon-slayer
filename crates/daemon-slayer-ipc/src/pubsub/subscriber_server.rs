use daemon_slayer_core::{
    async_trait,
    server::{BackgroundService, ServiceContext},
    BoxedError, FutureExt,
};

use crate::Codec;
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use super::{subscriber::Subscriber, SubscriberClient};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Unpin + 'static,
{
    app_id: String,
    codec: Codec,
    subscriber_tx: mpsc::Sender<(Vec<T>, mpsc::Sender<(T, M)>)>,
    subscriber_rx: mpsc::Receiver<(Vec<T>, mpsc::Sender<(T, M)>)>,
}

impl<T, M> SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    pub fn new(app_id: impl Into<String>, codec: Codec) -> Self {
        let (subscriber_tx, subscriber_rx) = mpsc::channel(32);
        Self {
            app_id: app_id.into(),
            codec,
            subscriber_tx,
            subscriber_rx,
        }
    }

    pub fn get_client(&self) -> SubscriberClient<T, M> {
        SubscriberClient::new(self.subscriber_tx.clone())
    }
}

#[async_trait]
impl<T, M> BackgroundService for SubscriberServer<T, M>
where
    T: FromStr + Display + Clone + Debug + Send + 'static,
    <T as FromStr>::Err: Debug + Send,
    M: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + Send + Debug + Unpin + 'static,
{
    fn name<'a>() -> &'a str {
        "subscriber_server"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut subscriber_handles = vec![];
        while let Ok(Some((topics, tx))) = self
            .subscriber_rx
            .recv()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            let subscriber = Subscriber::new(&self.app_id, tx, topics, self.codec.clone()).await;
            let cancellation_token = context.cancellation_token();
            subscriber_handles.push(tokio::spawn(async move {
                subscriber.run(cancellation_token).await;
            }));
        }

        Ok(())
    }
}
