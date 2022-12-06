use std::{env::current_exe, error::Error, path::PathBuf};

use daemon_slayer::{
    cli::{
        clap::{Arg, Command},
        Cli, InputState,
    },
    client::{
        cli::ClientCliProvider,
        config::{ServiceAccess, Trustee, WindowsConfig},
        Level, Manager, ServiceManager,
    },
    console::{cli::ConsoleCliProvider, Console},
    error_handler::{cli::ErrorHandlerCliProvider, ErrorHandler},
    health_check::{cli::HealthCheckCliProvider, HttpHealthCheck, HttpRequestType, IpcHealthCheck},
    logging::{
        cli::LoggingCliProvider, tracing_subscriber::util::SubscriberInitExt, LoggerBuilder,
    },
};

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let logger_builder = LoggerBuilder::new("daemon_slayer_server");
    let logging_provider = LoggingCliProvider::new(logger_builder);
    let manager = ServiceManager::builder("daemon_slayer_server")
        .with_description("test service")
        .with_program(current_exe().unwrap().parent().unwrap().join("server"))
        .with_service_level(if cfg!(windows) {
            Level::System
        } else {
            Level::User
        })
        .with_windows_config(WindowsConfig::default().with_additional_access(
            Trustee::CurrentUser,
            ServiceAccess::Start | ServiceAccess::Stop | ServiceAccess::ChangeConfig,
        ))
        .with_args(["run"])
        .build()?;

    let health_check = IpcHealthCheck::new("daemon_slayer_server");

    let console = Console::new(manager.clone()).with_health_check(Box::new(health_check.clone()));
    let cli = Cli::builder()
        .with_default_client_commands()
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(HealthCheckCliProvider::new(health_check))
        .with_provider(logging_provider.clone())
        .with_provider(ErrorHandlerCliProvider::default())
        .initialize();

    let (logger, _guard) = logging_provider.get_logger();
    logger.init();

    cli.handle_input().await;
    Ok(())
}
