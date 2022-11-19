use daemon_slayer_ipc::{
    PublisherClient, PublisherServer, ServiceFactory, Subscriber, SubscriberServer, TwoWayMessage,
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

    type Codec =
        Bincode<TwoWayMessage<Self::Req, Self::Resp>, TwoWayMessage<Self::Req, Self::Resp>>;

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
    fn make_codec(&self) -> Self::Codec {
        Bincode::default()
    }
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

#[tokio::main]
async fn main() {
    // let rpc = daemon_slayer_ipc::RpcService::new("supertest".to_owned(), PingFactory {});
    // rpc.spawn_server();
    // tokio::time::sleep(Duration::from_millis(100)).await;
    // let client = rpc.get_client().await;
    // client.ping(context::current()).await;

    PublisherServer {
        clients: Arc::new(std::sync::Mutex::new(HashMap::new())),
        subscriptions: Arc::new(std::sync::RwLock::new(HashMap::new())),
    }
    .start()
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let _subscriber0 =
        SubscriberServer::connect(vec!["calculus".into(), "cool shorts".into()]).await;

    let _subscriber1 =
        SubscriberServer::connect(vec!["cool shorts".into(), "history".into()]).await;

    let bind_addr = format!("\\\\.\\pipe\\test_pubsub_publisher");
    let mut endpoint = Endpoint::connect(bind_addr).await.unwrap();

    let mut codec_builder = LengthDelimitedCodec::builder();
    let framed = codec_builder
        .max_frame_length(usize::MAX)
        .new_framed(endpoint);
    let transport = serde_transport::new(framed, Bincode::default());
    let publisher = PublisherClient::new(client::Config::default(), transport).spawn();

    publisher
        .publish(context::current(), "calculus".into(), "sqrt(2)".into())
        .await;

    publisher
        .publish(
            context::current(),
            "cool shorts".into(),
            "hello to all".into(),
        )
        .await;

    publisher
        .publish(context::current(), "history".into(), "napoleon".to_string())
        .await;

    drop(_subscriber0);

    publisher
        .publish(
            context::current(),
            "cool shorts".into(),
            "hello to who?".into(),
        )
        .await;
}
