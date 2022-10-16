#[maybe_async_cfg::maybe(
    idents(Service, EventStore),
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait EventService: crate::service::Service {
    type EventStoreImpl: crate::event_store::EventStore;

    fn get_event_store(&mut self) -> Self::EventStoreImpl;
}
