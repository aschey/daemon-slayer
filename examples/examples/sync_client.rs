use daemon_slayer::cli::Action;
use daemon_slayer::cli::CliSync;
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
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
    let (logger, _guard) = cli.configure_logger().build()?;
    logger.init();
    cli.configure_error_handler().install()?;
    cli.handle_input()?;
    Ok(())
}
