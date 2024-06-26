use std::net::SocketAddr;

use daemon_slayer::core::socket_activation::ActivationSocketConfig;
use daemon_slayer::core::{CommandArg, Label};

pub fn label() -> Label {
    "com.example.daemon_slayer_mdns"
        .parse()
        .expect("Should parse the label")
}

pub fn run_argument() -> CommandArg {
    "run".parse().expect("Should parse the run argument")
}

pub const SOCKET_NAME: &str = "test_socket";
pub fn sockets(port: u32) -> Vec<ActivationSocketConfig> {
    vec![ActivationSocketConfig::new_tcp(
        SOCKET_NAME,
        format!("0.0.0.0:{port}").parse::<SocketAddr>().unwrap(),
    )]
}
