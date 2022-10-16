#[maybe_async_cfg::maybe(
    idents(Receiver),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
pub trait EventStore {
    type Item: Send;
    fn subscribe_events(&self) -> Box<dyn crate::receiver::Receiver<Item = Self::Item>>;
}
