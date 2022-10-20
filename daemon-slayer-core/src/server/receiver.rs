#[async_trait::async_trait]
pub trait Receiver: Send {
    type Item: Send;
    async fn recv(&mut self) -> Option<Self::Item>;
}

#[async_trait::async_trait]
impl<T: Send + Clone> Receiver for tokio::sync::broadcast::Receiver<T> {
    type Item = T;
    async fn recv(&mut self) -> Option<Self::Item> {
        self.recv().await.ok()
    }
}
