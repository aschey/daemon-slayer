use super::EventStore;

pub trait EventService: crate::server::background_service::BackgroundService {
    type EventStoreImpl: EventStore;

    fn get_event_store(&mut self) -> Self::EventStoreImpl;
}
