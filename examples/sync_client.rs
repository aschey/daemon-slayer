use daemon_slayer::cli::{Action, CliHandlerSync, ClientCliSync};
use daemon_slayer::client::{Manager, ServiceManager};
use std::time::{Duration, Instant};
use tracing::info;

pub fn main() {
    let manager = ServiceManager::builder("daemon_slayer_sync_server".to_owned())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = ClientCliSync::new(manager);
    cli.handle_input().unwrap();
}
