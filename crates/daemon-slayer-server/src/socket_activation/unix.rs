#[cfg(target_os = "macos")]
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixListener;

use daemon_slayer_core::socket_activation::{ActivationSocketConfig, SocketType};
use futures::future;
use parity_tokio_ipc::Endpoint;
use tokio::net::{TcpListener, UdpSocket};

use super::{
    create_sockets, to_hash_map, SocketActivationError, SocketActivationResult, SocketResult,
};

pub async fn get_activation_sockets(
    socket_config: Vec<ActivationSocketConfig>,
) -> Result<SocketActivationResult, SocketActivationError> {
    #[cfg(target_os = "linux")]
    let fds: Vec<_> = sd_listen_fds::get()
        .map_err(|e| SocketActivationError::UnableToLoad(e.to_string()))?
        .into_iter()
        .map(|r| r.1)
        .collect();

    #[cfg(target_os = "macos")]
    let fds: Result<Vec<_>, _> = socket_config
        .iter()
        .filter_map(|s| match raunch::activate_socket(s.name()) {
            Ok(fds) => Some(Ok(fds)),
            Err(raunch::Error::NotManaged) => None,
            Err(e) => Some(Err(SocketActivationError::UnableToLoad(e.to_string()))),
        })
        .collect();
    #[cfg(target_os = "macos")]
    let fds: Vec<_> = fds?
        .into_iter()
        .flatten()
        .map(|fd| unsafe { OwnedFd::from_raw_fd(fd) })
        .collect();

    let activated = !fds.is_empty();
    let supplied = socket_config.len();
    let returned = fds.len();
    if activated && fds.len() != socket_config.len() {
        return Err(SocketActivationError::Mismatch { supplied, returned });
    }

    if activated {
        let sockets = future::try_join_all(fds.into_iter().zip(socket_config.into_iter()).map(
            |(fd, config)| async {
                Ok((
                    config.name().to_owned(),
                    create_activated_socket(fd, config).await?,
                ))
            },
        ))
        .await?;

        Ok(SocketActivationResult {
            sockets: to_hash_map(sockets),
            is_activated: true,
        })
    } else {
        let sockets = create_sockets(socket_config).await?;
        Ok(SocketActivationResult {
            sockets: to_hash_map(sockets),
            is_activated: false,
        })
    }
}

async fn create_activated_socket(
    fd: OwnedFd,
    config: ActivationSocketConfig,
) -> Result<SocketResult, SocketActivationError> {
    Ok(match config.socket_type() {
        SocketType::Ipc => {
            let std_listener = UnixListener::from(fd);
            std_listener
                .set_nonblocking(true)
                .map_err(SocketActivationError::CreationFailure)?;
            SocketResult::Ipc(
                Endpoint::from_std_listener(std_listener)
                    .map_err(SocketActivationError::CreationFailure)?,
            )
        }

        SocketType::Tcp => {
            let std_listener = std::net::TcpListener::from(fd);
            std_listener
                .set_nonblocking(true)
                .map_err(SocketActivationError::CreationFailure)?;
            SocketResult::Tcp(
                TcpListener::from_std(std_listener)
                    .map_err(SocketActivationError::CreationFailure)?,
            )
        }

        SocketType::Udp => {
            let std_listener = std::net::UdpSocket::from(fd);
            std_listener
                .set_nonblocking(true)
                .map_err(SocketActivationError::CreationFailure)?;
            SocketResult::Udp(
                UdpSocket::from_std(std_listener)
                    .map_err(SocketActivationError::CreationFailure)?,
            )
        }
    })
}
