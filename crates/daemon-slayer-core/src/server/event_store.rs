use std::pin::Pin;

use futures::Stream;

pub trait EventStore {
    type Item: Send;
    fn subscribe_events(&self) -> Pin<Box<dyn Stream<Item = Self::Item> + Send>>;
}
