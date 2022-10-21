use std::{env::current_exe, error::Error, path::PathBuf};

use daemon_slayer::{
    client::{Level, Manager, ServiceManager},
    error_handler::ErrorHandler,
    logging::{tracing_subscriber::util::SubscriberInitExt, LoggerBuilder},
};

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (logger, _guard) = LoggerBuilder::for_server("daemon_slayer_axum")
        .with_default_log_level(tracing::Level::TRACE)
        .build()?;

    logger.init();
    ErrorHandler::for_server().install()?;

    let manager = ServiceManager::builder("daemon_slayer_axum")
        .with_description("test service")
        .with_program(current_exe().unwrap().parent().unwrap().join("http_server"))
        .with_service_level(if cfg!(windows) {
            Level::System
        } else {
            Level::User
        })
        .with_args(["run"])
        .build()?;

    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png"));
    daemon_slayer::tray::start(&path, manager);
    Ok(())
}
