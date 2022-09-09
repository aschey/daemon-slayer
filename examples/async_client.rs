use std::env::args;
use std::error::Error;
use std::time::{Duration, Instant};

use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{Action, CliHandlerAsync, Command};

use daemon_slayer_cli::CliAsync;
use futures::{SinkExt, StreamExt};

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let manager = ServiceManager::builder("daemon_slayer_async_server")
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = CliAsync::new_client(manager);
    cli.handle_input().await?;
    Ok(())
}
