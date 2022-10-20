use crate::server::{EventStore, Receiver};

#[derive(Clone)]
pub struct BroadcastEventStore<T> {
    tx: tokio::sync::broadcast::Sender<T>,
}

impl<T: Send> BroadcastEventStore<T> {
    pub fn new(tx: tokio::sync::broadcast::Sender<T>) -> Self {
        Self { tx }
    }
}

impl<T: Send + Clone + 'static> EventStore for BroadcastEventStore<T> {
    type Item = T;
    fn subscribe_events(&self) -> Box<dyn Receiver<Item = Self::Item>> {
        Box::new(self.tx.subscribe())
    }
}
