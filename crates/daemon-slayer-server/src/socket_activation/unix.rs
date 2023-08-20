#[cfg(target_os = "macos")]
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixListener;

use daemon_slayer_core::socket_activation::{ActivationSocketConfig, SocketType};
use parity_tokio_ipc::{Endpoint, IpcEndpoint, IpcSecurity, SecurityAttributes};
use tokio::net::{TcpListener, UdpSocket};

use super::SocketResult;

pub struct ActivationSockets {
    fds: Vec<OwnedFd>,
    socket_config: Vec<ActivationSocketConfig>,
    pos: usize,
}

impl ActivationSockets {
    pub fn get(socket_config: Vec<ActivationSocketConfig>) -> Self {
        #[cfg(target_os = "linux")]
        let fds = sd_listen_fds::get()
            .unwrap()
            .into_iter()
            .map(|r| r.1)
            .collect();
        #[cfg(target_os = "macos")]
        let fds = socket_config
            .iter()
            .map(|s| raunch::activate_socket(s.name()).unwrap())
            .flatten()
            .map(|r| unsafe { OwnedFd::from_raw_fd(r) })
            .collect();

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
                    let std_listener = UnixListener::from(fd.try_clone().unwrap());
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
                    let std_listener = std::net::TcpListener::from(fd.try_clone().unwrap());
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
                    let std_listener = std::net::UdpSocket::from(fd.try_clone().unwrap());
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
