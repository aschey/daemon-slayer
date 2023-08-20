use std::net::SocketAddr;

use parity_tokio_ipc::{ConnectionId, IntoIpcPath};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketType {
    Ipc,
    Tcp,
    Udp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivationSocketConfig {
    name: String,
    addr: String,
    socket_type: SocketType,
}

impl ActivationSocketConfig {
    pub fn new_ipc(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            addr: ConnectionId(id.into())
                .into_ipc_path()
                .to_string_lossy()
                .to_string(),
            socket_type: SocketType::Ipc,
        }
    }

    pub fn new_tcp(name: impl Into<String>, addr: impl Into<SocketAddr>) -> Self {
        let addr: SocketAddr = addr.into();
        Self {
            name: name.into(),
            addr: addr.to_string(),
            socket_type: SocketType::Tcp,
        }
    }

    pub fn new_udp(name: impl Into<String>, addr: impl Into<SocketAddr>) -> Self {
        let addr: SocketAddr = addr.into();
        Self {
            name: name.into(),
            addr: addr.to_string(),
            socket_type: SocketType::Udp,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn socket_type(&self) -> SocketType {
        self.socket_type
    }
}
