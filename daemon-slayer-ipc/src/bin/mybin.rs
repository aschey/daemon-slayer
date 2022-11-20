use bytes::{Bytes, BytesMut};
use daemon_slayer_ipc::{
    get_publisher, Codec, PublisherClient, PublisherServer, ServiceFactory, Subscriber,
    SubscriberServer, TwoWayMessage,
};
use futures::Future;
use parity_tokio_ipc::Endpoint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tarpc::transport::channel::UnboundedChannel;
use tarpc::{
    client, context, serde_transport, tokio_serde::formats::Bincode,
    tokio_util::codec::LengthDelimitedCodec, transport, ClientMessage, Response,
};
use tokio::sync::{Mutex, RwLock};

#[tarpc::service]
pub trait Ping {
    async fn hello(name: String) -> String;
    async fn ping();
    async fn pong(count: u64) -> u64;
}

#[derive(Clone)]
struct PingServer {
    peer: PingClient,
    count: Arc<Mutex<u64>>,
}

#[derive(Clone)]
struct PingFactory {}

impl daemon_slayer_ipc::ServiceFactory for PingFactory {
    type Client = PingClient;
    type Service = ServePing<PingServer>;
    type Req = PingRequest;
    type Resp = PingResponse;

    // type Codec =
    //     Bincode<TwoWayMessage<Self::Req, Self::Resp>, TwoWayMessage<Self::Req, Self::Resp>>;

    fn make_service(&self, client: Self::Client) -> Self::Service {
        PingServer {
            peer: client,
            count: Arc::new(Mutex::new(0)),
        }
        .serve()
    }
    fn make_client(
        &self,
        chan: UnboundedChannel<Response<Self::Resp>, ClientMessage<Self::Req>>,
    ) -> Self::Client {
        PingClient::new(client::Config::default(), chan).spawn()
    }
    // fn make_codec(&self) -> Self::Codec {
    //     Bincode::default()
    // }
}

#[tarpc::server]
impl Ping for PingServer {
    async fn hello(self, _: tarpc::context::Context, name: String) -> String {
        return format!("Hello {name}");
    }

    async fn ping(mut self, _: context::Context) {
        println!("ping {}", self.count.lock().await);
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut count = self.count.lock().await;

        *count = self.peer.pong(context::current(), *count).await.unwrap();
    }

    async fn pong(mut self, _: context::Context, count: u64) -> u64 {
        println!("pong {}", count);
        return count + 1;
    }
}

#[derive(strum_macros::Display)]
enum MyTopic {
    Foo,
    Bar,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, strum_macros::Display)]
enum MyMessage {
    Yes(String),
    No,
}

#[derive(Clone)]
struct MySubscriber {}

#[async_trait::async_trait]
impl daemon_slayer_ipc::PubSubSubscriber for MySubscriber {
    type Topic = MyTopic;
    type Message = MyMessage;

    async fn topics(&self) -> Vec<MyTopic> {
        vec![MyTopic::Foo, MyTopic::Bar]
    }
    async fn on_event(&self, topic: String, message: Self::Message) {
        println!("got {topic} {message}");
    }
}

// #[derive(Clone)]
// struct MyCodec {}

// impl daemon_slayer_ipc::CodecFactory for MyCodec {
//     type Encode = String;
//     type Decode = String;
//     type Codec = Bincode<Self::Encode, Self::Decode>;

//     fn make_codec(&self) -> Self::Codec {
//         Bincode::default()
//     }
// }

#[tokio::main]
async fn main() {
    let app_id = "supertest";
    let codec = Codec::Cbor;
    let rpc = daemon_slayer_ipc::RpcService::new(app_id, PingFactory {}, codec.clone());
    rpc.spawn_server();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let client = rpc.get_client().await;
    client.ping(context::current()).await;

    PublisherServer::new(&app_id).start().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let _subscriber0 = SubscriberServer::connect(&app_id, MySubscriber {}, codec.clone()).await;

    // let _subscriber1 =
    //     SubscriberServer::connect(&app_id, vec!["cool shorts".into(), "history".into()]).await;

    let mut publisher = get_publisher::<MyTopic, MyMessage>(app_id, codec).await; //PublisherClient::new(client::Config::default(), transport).spawn();

    publisher.publish(MyTopic::Bar, MyMessage::No).await;

    publisher
        .publish(MyTopic::Foo, MyMessage::Yes("hi".to_owned()))
        .await;

    // publisher.publish("history".into(), "napoleon".into()).await;

    // publisher
    //     .publish("cool shorts".into(), "hello to who?".into())
    //     .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
}
