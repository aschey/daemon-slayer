use bytes::Bytes;
mod subscription;

mod subscriber_server;
pub use subscriber_server::*;

mod publisher_server;
pub use publisher_server::*;

mod subscriber;
pub use subscriber::*;

mod publisher;
pub use publisher::*;

mod subscriber_handle;
pub use subscriber_handle::*;

#[tarpc::service]
pub trait SubscriberService {
    async fn topics() -> Vec<String>;
    async fn receive(topic: String, message: Bytes);
    async fn id() -> u32;
}

#[tarpc::service]
pub trait PublisherService {
    async fn publish(topic: String, message: Bytes);
}
