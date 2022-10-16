#[maybe_async_cfg::maybe(
    sync(feature = "blocking"),
    async(feature = "async-tokio", async_trait::async_trait)
)]
pub trait Receiver: Send {
    type Item: Send;
    async fn recv(&mut self) -> Option<Self::Item>;
}

#[cfg(feature = "async-tokio")]
#[async_trait::async_trait]
impl<T: Send + Clone> ReceiverAsync for tokio::sync::broadcast::Receiver<T> {
    type Item = T;
    async fn recv(&mut self) -> Option<Self::Item> {
        self.recv().await.ok()
    }
}

#[cfg(feature = "blocking")]
impl<T: Send + Sync + Clone> ReceiverSync for bus::BusReader<T> {
    type Item = T;
    fn recv(&mut self) -> Option<Self::Item> {
        self.recv().ok()
    }
}
