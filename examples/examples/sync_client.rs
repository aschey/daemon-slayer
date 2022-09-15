use daemon_slayer::cli::Action;
use daemon_slayer::cli::CliSync;
use daemon_slayer::client::{Manager, ServiceManager};
use std::error::Error;
use std::time::{Duration, Instant};
use tracing::info;

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ServiceManager::builder("daemon_slayer_sync_server".to_owned())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = CliSync::for_client(manager);
    cli.handle_input()?;
    Ok(())
}
