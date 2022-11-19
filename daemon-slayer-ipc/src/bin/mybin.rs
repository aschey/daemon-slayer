use daemon_slayer_ipc::{ServiceFactory, TwoWayMessage};
use futures::Future;
use parity_tokio_ipc::Endpoint;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tarpc::transport::channel::UnboundedChannel;
use tarpc::{
    client, context, serde_transport, tokio_serde::formats::Bincode,
    tokio_util::codec::LengthDelimitedCodec, transport, ClientMessage, Response,
};
use tokio::sync::Mutex;

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
    let rpc = daemon_slayer_ipc::RpcService::new("supertest".to_owned(), PingFactory {});
    rpc.spawn_server();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let client = rpc.get_client().await;
    client.ping(context::current()).await;
}
