use std::env::args;
use std::error::Error;
use std::time::{Duration, Instant};

use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{Action, Command};

use daemon_slayer::cli::CliAsync;
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use futures::{SinkExt, StreamExt};

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ServiceManager::builder("daemon_slayer_async_server")
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = CliAsync::for_client(manager);
    let (logger, _guard) = cli.configure_logger().build()?;
    logger.init();
    cli.configure_error_handler().install()?;

    cli.handle_input().await?;
    Ok(())
}
