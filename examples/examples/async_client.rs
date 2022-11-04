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
    error_handler::ErrorHandler,
    health_check::{cli::HealthCheckCliProvider, HttpHealthCheck, HttpRequestType, IpcHealthCheck},
    logging::tracing_subscriber::util::SubscriberInitExt,
};

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (logger, _guard) =
        daemon_slayer::logging::LoggerBuilder::for_client("daemon_slayer_async_server").build()?;
    ErrorHandler::for_client().install()?;
    let manager = ServiceManager::builder("daemon_slayer_async_server")
        .with_description("test service")
        .with_program(
            current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join("async_server"),
        )
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
    logger.init();

    let health_check = IpcHealthCheck::new("daemon_slayer_async_server");

    let mut console = Console::new(manager.clone());
    console.add_health_check(Box::new(health_check.clone()));
    let (cli, command) = Cli::builder()
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(HealthCheckCliProvider::new(health_check))
        .build();

    let matches = command.get_matches();
    cli.handle_input(&matches).await;
    Ok(())
}
