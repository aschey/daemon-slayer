use crate::server::blocking::{EventStore, Receiver};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct BroadcastEventStore<T> {
    bus: Arc<Mutex<bus::Bus<T>>>,
}

impl<T: Send> BroadcastEventStore<T> {
    pub fn new(bus: Arc<Mutex<bus::Bus<T>>>) -> Self {
        Self { bus }
    }
}

impl<T: Send + Sync + Clone + 'static> EventStore for BroadcastEventStore<T> {
    type Item = T;
    fn subscribe_events(&self) -> Box<dyn Receiver<Item = Self::Item>> {
        Box::new(self.bus.lock().unwrap().add_rx())
    }
}
