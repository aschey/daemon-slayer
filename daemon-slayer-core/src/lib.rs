#[async_trait::async_trait]
pub trait Service: Send {
    type Builder;
    type Client;

    async fn run_service(builder: Self::Builder) -> Self;

    fn get_client(&mut self) -> Self::Client;

    async fn stop(mut self);
}

pub trait EventService: Service {
    type Event;

    fn get_event_store(&mut self) -> Box<dyn EventStore<Item = Self::Event>>;
}

#[derive(Clone)]
pub struct BroadcastEventStore<T> {
    tx: tokio::sync::broadcast::Sender<T>,
}

impl<T: Send + Clone + 'static> EventStore for BroadcastEventStore<T> {
    type Item = T;
    fn subscribe_events(&self) -> Box<dyn Receiver<Item = Self::Item>> {
        Box::new(self.tx.subscribe())
    }
}

#[async_trait::async_trait]
impl<T: Send + Clone> Receiver for tokio::sync::broadcast::Receiver<T> {
    type Item = T;
    async fn recv(&mut self) -> Option<Self::Item> {
        self.recv().await.ok()
    }
}

impl<T: Send> BroadcastEventStore<T> {
    pub fn new(tx: tokio::sync::broadcast::Sender<T>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
pub trait Receiver: Send {
    type Item: Send;
    async fn recv(&mut self) -> Option<Self::Item>;
}

pub trait EventStore {
    type Item: Send;
    fn subscribe_events(&self) -> Box<dyn Receiver<Item = Self::Item>>;
}
