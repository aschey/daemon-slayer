use daemon_slayer_core::socket_activation::{ActivationSocketConfig, SocketType};
use parity_tokio_ipc::{Endpoint, IpcEndpoint, IpcSecurity, SecurityAttributes};
use tokio::net::{TcpListener, UdpSocket};

use super::SocketResult;

pub struct ActivationSockets {
    socket_config: Vec<ActivationSocketConfig>,
    pos: usize,
}

impl ActivationSockets {
    pub fn get(socket_config: Vec<ActivationSocketConfig>) -> Self {
        Self {
            pos: 0,
            socket_config,
        }
    }

    pub async fn next(&mut self) -> Option<SocketResult> {
        let current_pos = self.pos;
        self.pos += 1;
        if let Some(config) = self.socket_config.get(current_pos) {
            match config.socket_type() {
                SocketType::Ipc => {
                    let mut endpoint = Endpoint::new(config.addr()).unwrap();
                    endpoint.set_security_attributes(
                        SecurityAttributes::allow_everyone_create().unwrap(),
                    );
                    return Some(SocketResult::Ipc(endpoint.incoming().unwrap()));
                }
                SocketType::Tcp => {
                    return Some(SocketResult::Tcp(
                        TcpListener::bind(config.addr()).await.unwrap(),
                    ));
                }
                SocketType::Udp => {
                    return Some(SocketResult::Udp(
                        UdpSocket::bind(config.addr()).await.unwrap(),
                    ));
                }
            }
        }
        None
    }
}
