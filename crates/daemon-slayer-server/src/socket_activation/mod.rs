#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::collections::HashMap;
use std::path::PathBuf;

use daemon_slayer_core::socket_activation::{ActivationSocketConfig, SocketType};
use futures::future;
use parity_tokio_ipc::{
    Endpoint, IpcEndpoint, IpcSecurity, IpcStream, OnConflict, SecurityAttributes,
};
use tokio::net::{TcpListener, UdpSocket};
#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
pub use windows::*;

pub enum SocketResult {
    Ipc(IpcStream),
    Tcp(TcpListener),
    Udp(UdpSocket),
}

pub struct SocketActivationResult {
    pub sockets: HashMap<String, Vec<SocketResult>>,
    pub is_activated: bool,
}

pub(super) async fn create_socket(config: ActivationSocketConfig) -> SocketResult {
    match config.socket_type() {
        SocketType::Ipc => {
            let mut endpoint =
                Endpoint::new(PathBuf::from(config.addr()), OnConflict::Overwrite).unwrap();
            endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());
            SocketResult::Ipc(endpoint.incoming().unwrap())
        }
        SocketType::Tcp => SocketResult::Tcp(TcpListener::bind(config.addr()).await.unwrap()),
        SocketType::Udp => SocketResult::Udp(UdpSocket::bind(config.addr()).await.unwrap()),
    }
}

pub(super) fn to_hash_map(
    sockets: Vec<(String, SocketResult)>,
) -> HashMap<String, Vec<SocketResult>> {
    sockets
        .into_iter()
        .fold(HashMap::new(), |mut map, (name, socket)| {
            if let Some(val) = map.get_mut(&name) {
                val.push(socket);
            } else {
                map.insert(name, vec![socket]);
            }
            map
        })
}

pub(super) async fn create_sockets(
    socket_config: Vec<ActivationSocketConfig>,
) -> Vec<(String, SocketResult)> {
    future::join_all(
        socket_config
            .into_iter()
            .map(|config| async { (config.name().to_owned(), create_socket(config).await) }),
    )
    .await
}
