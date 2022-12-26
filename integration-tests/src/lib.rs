use std::net::SocketAddr;

use confique::Config;
use daemon_slayer::{client, core::Label};

#[derive(Debug, Config, Default, Clone)]
pub struct TestConfig {
    #[config(nested)]
    client_config: client::config::UserConfig,
}

impl AsRef<client::config::UserConfig> for TestConfig {
    fn as_ref(&self) -> &client::config::UserConfig {
        &self.client_config
    }
}

pub fn label() -> Label {
    "com.test.daemon_slayer_test".parse().unwrap()
}

pub fn address() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 3002))
}

pub fn address_string() -> String {
    format!("http://{}", address())
}
