use std::sync::Arc;

use daemon_slayer::ipc::rpc::{RpcService, ServiceProvider};
use daemon_slayer::ipc::Codec;
use tarpc::context::{self, Context};
use tarpc::{client, transport::channel::UnboundedChannel, ClientMessage, Response};
use tokio::sync::Mutex;
use tracing::info;

#[derive(
    Debug,
    Clone,
    strum_macros::EnumString,
    strum_macros::Display,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Topic {
    Topic1,
    Topic2,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Message {
    Message1,
    Message2,
}

#[tarpc::service]
pub trait Ping {
    async fn ping();
    async fn pong(count: u64) -> u64;
}

#[derive(Clone)]
pub struct PingServer {
    count: Arc<Mutex<u64>>,
    peer: PingClient,
}

#[tarpc::server]
impl Ping for PingServer {
    async fn ping(self, _: Context) {
        info!("Sending ping");
        let mut count = self.count.lock().await;
        *count = self.peer.pong(context::current(), *count).await.unwrap();
        info!("Got pong: {count}");
    }

    async fn pong(self, _: Context, count: u64) -> u64 {
        count + 1
    }
}

#[derive(Clone, Default)]
pub struct PingProvider {
    count: Arc<Mutex<u64>>,
}

impl ServiceProvider for PingProvider {
    type Req = PingRequest;
    type Resp = PingResponse;
    type Client = PingClient;
    type Service = ServePing<PingServer>;

    fn get_service(&self, client: Self::Client) -> Self::Service {
        PingServer {
            count: self.count.clone(),
            peer: client,
        }
        .serve()
    }

    fn get_client(
        &self,
        chan: UnboundedChannel<Response<Self::Resp>, ClientMessage<Self::Req>>,
    ) -> Self::Client {
        PingClient::new(client::Config::default(), chan).spawn()
    }
}

pub fn get_rpc_service() -> RpcService<PingProvider> {
    RpcService::new("daemon_slayer_ipc", PingProvider::default(), Codec::Bincode)
}
