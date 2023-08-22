#[cfg(target_os = "macos")]
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixListener;

use daemon_slayer_core::socket_activation::{ActivationSocketConfig, SocketType};
use futures::future;
use parity_tokio_ipc::Endpoint;
#[cfg(target_os = "macos")]
use tap::TapFallible;
use tokio::net::{TcpListener, UdpSocket};
#[cfg(target_os = "macos")]
use tracing::warn;

use super::{create_sockets, to_hash_map, SocketActivationResult, SocketResult};

pub async fn get_activation_sockets(
    socket_config: Vec<ActivationSocketConfig>,
) -> SocketActivationResult {
    #[cfg(target_os = "linux")]
    let fds: Vec<_> = sd_listen_fds::get()
        .unwrap()
        .into_iter()
        .map(|r| r.1)
        .collect();
    #[cfg(target_os = "macos")]
    let fds = socket_config
        .iter()
        .filter_map(|s| {
            raunch::activate_socket(s.name())
                .tap_err(|e| {
                    warn!(
                        "unable to retrieve socket info for {} from launchd: {e:?}. This is \
                         expected if the process is not running under launchd.",
                        s.name()
                    )
                })
                .ok()
        })
        .flatten()
        .map(|r| unsafe { OwnedFd::from_raw_fd(r) })
        .collect();
    let activated = !fds.is_empty();
    if activated && fds.len() != socket_config.len() {
        panic!("mismatch");
    }

    if activated {
        let sockets = future::join_all(fds.into_iter().zip(socket_config.into_iter()).map(
            |(fd, config)| async {
                (
                    config.name().to_owned(),
                    create_activated_socket(fd, config).await,
                )
            },
        ))
        .await;

        SocketActivationResult {
            sockets: to_hash_map(sockets),
            is_activated: true,
        }
    } else {
        let sockets = create_sockets(socket_config).await;
        SocketActivationResult {
            sockets: to_hash_map(sockets),
            is_activated: false,
        }
    }
}

async fn create_activated_socket(fd: OwnedFd, config: ActivationSocketConfig) -> SocketResult {
    match config.socket_type() {
        SocketType::Ipc => {
            let std_listener = UnixListener::from(fd.try_clone().unwrap());
            SocketResult::Ipc(Endpoint::from_std_listener(std_listener).unwrap())
        }

        SocketType::Tcp => {
            let std_listener = std::net::TcpListener::from(fd.try_clone().unwrap());
            std_listener.set_nonblocking(true).unwrap();
            SocketResult::Tcp(TcpListener::from_std(std_listener).unwrap())
        }

        SocketType::Udp => {
            let std_listener = std::net::UdpSocket::from(fd.try_clone().unwrap());
            std_listener.set_nonblocking(true).unwrap();
            SocketResult::Udp(UdpSocket::from_std(std_listener).unwrap())
        }
    }
}
