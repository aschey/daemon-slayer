use super::EventStore;

pub trait EventService: crate::server::service::Service {
    type EventStoreImpl: EventStore;

    fn get_event_store(&mut self) -> Self::EventStoreImpl;
}
