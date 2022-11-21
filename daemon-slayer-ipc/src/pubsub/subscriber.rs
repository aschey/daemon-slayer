use std::fmt::Display;

#[async_trait::async_trait]
pub trait Subscriber: Clone + Send + Sync + 'static {
    type Topic: Display;
    type Message: serde::Serialize
        + for<'de> serde::Deserialize<'de>
        + Clone
        + Send
        + Unpin
        + 'static;

    async fn topics(&self) -> Vec<Self::Topic>;
    async fn on_event(&self, topic: String, message: Self::Message);
}
