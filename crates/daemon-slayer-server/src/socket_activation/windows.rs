use std::path::PathBuf;

use daemon_slayer_core::socket_activation::{ActivationSocketConfig, SocketType};
use futures::future;
use parity_tokio_ipc::{Endpoint, IpcEndpoint, IpcSecurity, OnConflict, SecurityAttributes};
use tokio::net::{TcpListener, UdpSocket};

use super::{create_sockets, SocketResult, SocketType};

pub async fn get_activation_sockets(socket_config: Vec<ActivationSocketConfig>) -> SocketResult {
    let sockets = create_sockets(socket_config).await;
    SocketResult {
        sockets: to_hash_map(sockets),
        is_activated: false,
    }
}
