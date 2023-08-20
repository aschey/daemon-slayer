use std::os::fd::OwnedFd;
use std::os::unix::net::UnixListener;

use daemon_slayer_core::socket_activation::{ActivationSocketConfig, SocketType};
use parity_tokio_ipc::{Endpoint, IpcEndpoint, IpcSecurity, IpcStream, SecurityAttributes};
use tokio::net::{TcpListener, UdpSocket};

pub struct ActivationSockets {
    fds: Vec<(Option<String>, OwnedFd)>,
    socket_config: Vec<ActivationSocketConfig>,
    pos: usize,
}

pub enum SocketResult {
    Ipc(IpcStream),
    Tcp(TcpListener),
    Udp(UdpSocket),
}

impl ActivationSockets {
    pub fn get(socket_config: Vec<ActivationSocketConfig>) -> Self {
        let fds = sd_listen_fds::get().unwrap();
        Self {
            fds,
            pos: 0,
            socket_config,
        }
    }

    pub async fn next(&mut self) -> Option<SocketResult> {
        let current_pos = self.pos;
        self.pos += 1;
        if let Some(config) = self.socket_config.get(current_pos) {
            match (config.socket_type(), self.fds.get(current_pos)) {
                (SocketType::Ipc, Some(fd)) => {
                    let std_listener = UnixListener::from(fd.1.try_clone().unwrap());
                    return Some(SocketResult::Ipc(
                        Endpoint::from_std_listener(std_listener).unwrap(),
                    ));
                }
                (SocketType::Ipc, None) => {
                    let mut endpoint = Endpoint::new(config.addr());
                    endpoint.set_security_attributes(
                        SecurityAttributes::allow_everyone_create().unwrap(),
                    );
                    return Some(SocketResult::Ipc(endpoint.incoming().unwrap()));
                }
                (SocketType::Tcp, Some(fd)) => {
                    let std_listener = std::net::TcpListener::from(fd.1.try_clone().unwrap());
                    std_listener.set_nonblocking(true).unwrap();
                    return Some(SocketResult::Tcp(
                        TcpListener::from_std(std_listener).unwrap(),
                    ));
                }
                (SocketType::Tcp, None) => {
                    return Some(SocketResult::Tcp(
                        TcpListener::bind(config.addr()).await.unwrap(),
                    ));
                }
                (SocketType::Udp, Some(fd)) => {
                    let std_listener = std::net::UdpSocket::from(fd.1.try_clone().unwrap());
                    std_listener.set_nonblocking(true).unwrap();
                    return Some(SocketResult::Udp(
                        UdpSocket::from_std(std_listener).unwrap(),
                    ));
                }
                (SocketType::Udp, None) => {
                    return Some(SocketResult::Udp(
                        UdpSocket::bind(config.addr()).await.unwrap(),
                    ));
                }
            }
        }
        None
    }
}
