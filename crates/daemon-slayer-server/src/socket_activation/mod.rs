#[cfg(target_os = "macos")]
mod launchd;
#[cfg(target_os = "linux")]
mod systemd;

#[cfg(target_os = "macos")]
pub use launchd::*;
use parity_tokio_ipc::IpcStream;
#[cfg(target_os = "linux")]
pub use systemd::*;
use tokio::net::{TcpListener, UdpSocket};

pub enum SocketResult {
    Ipc(IpcStream),
    Tcp(TcpListener),
    Udp(UdpSocket),
}
