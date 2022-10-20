use super::Receiver;

pub trait EventStore {
    type Item: Send;
    fn subscribe_events(&self) -> Box<dyn Receiver<Item = Self::Item>>;
}
