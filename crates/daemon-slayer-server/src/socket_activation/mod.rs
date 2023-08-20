#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use parity_tokio_ipc::IpcStream;
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
