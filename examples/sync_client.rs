use daemon_slayer::cli::{Action, Cli, CliHandler};
use daemon_slayer::client::{Manager, ServiceManager};
use std::time::{Duration, Instant};
use tracing::info;

pub fn main() {
    let manager = ServiceManager::builder("daemon_slayer_test_service".to_owned())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = Cli::new(manager);

    cli.handle_input().unwrap();
}
