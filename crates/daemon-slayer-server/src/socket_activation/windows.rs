use daemon_slayer_core::socket_activation::ActivationSocketConfig;

use super::{create_sockets, to_hash_map, SocketActivationError, SocketActivationResult};

pub async fn get_activation_sockets(
    socket_config: Vec<ActivationSocketConfig>,
) -> Result<SocketActivationResult, SocketActivationError> {
    let sockets = create_sockets(socket_config).await?;
    Ok(SocketActivationResult {
        sockets: to_hash_map(sockets),
        is_activated: false,
    })
}
