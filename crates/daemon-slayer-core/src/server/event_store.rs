use std::pin::Pin;

use futures::{Stream, StreamExt};
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

pub trait EventStore {
    type Item: Send;
    fn subscribe_events(&self) -> Pin<Box<dyn Stream<Item = Self::Item> + Send>>;
}

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

pub struct DedupeEventStore<T> {
    inner: T,
}

impl<T> DedupeEventStore<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: EventStore + 'static> EventStore for DedupeEventStore<T>
where
    T::Item: Clone + Eq + Send + Sync,
{
    type Item = T::Item;
    fn subscribe_events(&self) -> Pin<Box<dyn Stream<Item = Self::Item> + Send>> {
        let inner_stream = self.inner.subscribe_events();
        let stream = async_stream::stream! {
            let mut last = None;
            for await value in inner_stream {
                if let Some(last_val) = last.as_ref() {
                    if last_val == &value {
                        continue;
                    }
                }
                last = Some(value.clone());
                yield value;
            }
        };
        stream.boxed()
    }
}
