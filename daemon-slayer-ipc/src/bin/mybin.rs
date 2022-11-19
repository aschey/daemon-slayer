use futures::Future;
use parity_tokio_ipc::Endpoint;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tarpc::{
    client, context, serde_transport, tokio_serde::formats::Bincode,
    tokio_util::codec::LengthDelimitedCodec, transport,
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
    daemon_slayer_ipc::RpcService::spawn_server(
        "supertest".to_owned(),
        |client_chan| {
            let client = PingClient::new(client::Config::default(), client_chan).spawn();
            PingServer {
                peer: client,
                count: Arc::new(Mutex::new(0)),
            }
            .serve()
        },
        Bincode::default,
    );
    let client = daemon_slayer_ipc::RpcService::get_client(
        "supertest".to_owned(),
        |client| {
            PingServer {
                peer: client,
                count: Arc::new(Mutex::new(0)),
            }
            .serve()
        },
        |chan| PingClient::new(client::Config::default(), chan).spawn(),
        Bincode::default,
    )
    .await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    println!(
        "{}",
        client
            .hello(context::current(), "bob".to_string())
            .await
            .unwrap()
    );
    client.ping(context::current()).await;
}
