use std::pin::Pin;

use futures::Stream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

use crate::server::EventStore;

#[derive(Clone)]
pub struct BroadcastEventStore<T> {
    tx: broadcast::Sender<T>,
}

impl<T: Send> BroadcastEventStore<T> {
    pub fn new(tx: broadcast::Sender<T>) -> Self {
        Self { tx }
    }
}

impl<T: Send + Clone + 'static> EventStore for BroadcastEventStore<T> {
    type Item = Result<T, BroadcastStreamRecvError>;
    fn subscribe_events(&self) -> Pin<Box<dyn Stream<Item = Self::Item> + Send>> {
        Box::pin(tokio_stream::wrappers::BroadcastStream::new(
            self.tx.subscribe(),
        ))
    }
}
