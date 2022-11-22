use bytes::Bytes;

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
