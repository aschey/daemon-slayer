use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;

use daemon_slayer::network::bytes::Bytes;
use daemon_slayer::network::futures::{SinkExt, StreamExt};
use daemon_slayer::network::tokio_util::codec::BytesCodec;
use daemon_slayer::network::tokio_util::udp::UdpFramed;
use daemon_slayer::network::{serde_json, ServiceMetadata};
use tokio::net::UdpSocket;

async fn test<M: ServiceMetadata>(metadata: M) {
    let sender = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let dest: SocketAddr = "255.255.255.255:34254".parse().unwrap();
    sender.set_broadcast(true).unwrap();
    let metadata = metadata.metadata();
    let mut framed = UdpFramed::new(sender, BytesCodec::new());
    let json_data = serde_json::to_string(&metadata).unwrap();
    framed
        .send((Bytes::from(json_data.clone()), dest))
        .await
        .unwrap();
    framed.send((Bytes::from(json_data), dest)).await.unwrap();
}

async fn test2<M: ServiceMetadata + Debug>() {
    let recv = UdpSocket::bind("0.0.0.0:34254").await.unwrap();
    recv.set_broadcast(true).unwrap();
    let mut framed = UdpFramed::new(recv, BytesCodec::new());
    let (data, _) = framed.next().await.unwrap().unwrap();

    let metadata: HashMap<String, String> = serde_json::from_slice(&data).unwrap();
    let data = M::from_metadata(metadata);
    println!("{:?}", data);
    let (data, _) = framed.next().await.unwrap().unwrap();

    let metadata: HashMap<String, String> = serde_json::from_slice(&data).unwrap();
    let data = M::from_metadata(metadata);
    println!("{:?}", data);
}

#[tokio::main]
async fn main() {
    let metadata = HashMap::from([("test".to_string(), "yes".to_string())]);

    tokio::spawn(async move { test(metadata).await });
    test2::<HashMap<String, String>>().await;
}
