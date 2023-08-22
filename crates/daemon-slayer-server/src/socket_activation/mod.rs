#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::collections::HashMap;
use std::io;
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

#[derive(Debug, thiserror::Error)]
pub enum SocketActivationError {
    #[error("Unable to load sockets from the service manager: {0}")]
    UnableToLoad(String),
    #[error(
        "There was a mismatch between the number of sockets returned by the service manager \
         ({returned}) and the number present in the supplied configuration ({supplied})"
    )]
    Mismatch { supplied: usize, returned: usize },
    #[error("Failed to create socket: {0}")]
    CreationFailure(io::Error),
}

pub struct SocketActivationResult {
    pub sockets: HashMap<String, Vec<SocketResult>>,
    pub is_activated: bool,
}

pub(super) async fn create_socket(
    config: ActivationSocketConfig,
) -> Result<SocketResult, SocketActivationError> {
    Ok(match config.socket_type() {
        SocketType::Ipc => {
            let mut endpoint = Endpoint::new(PathBuf::from(config.addr()), OnConflict::Overwrite)
                .map_err(SocketActivationError::CreationFailure)?;
            endpoint.set_security_attributes(
                SecurityAttributes::allow_everyone_create()
                    .map_err(SocketActivationError::CreationFailure)?,
            );
            SocketResult::Ipc(
                endpoint
                    .incoming()
                    .map_err(SocketActivationError::CreationFailure)?,
            )
        }
        SocketType::Tcp => SocketResult::Tcp(
            TcpListener::bind(config.addr())
                .await
                .map_err(SocketActivationError::CreationFailure)?,
        ),
        SocketType::Udp => SocketResult::Udp(
            UdpSocket::bind(config.addr())
                .await
                .map_err(SocketActivationError::CreationFailure)?,
        ),
    })
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
) -> Result<Vec<(String, SocketResult)>, SocketActivationError> {
    future::try_join_all(
        socket_config
            .into_iter()
            .map(|config| async { Ok((config.name().to_owned(), create_socket(config).await?)) }),
    )
    .await
}
