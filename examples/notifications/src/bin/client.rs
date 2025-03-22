use std::env::current_exe;

use daemon_slayer::cli::Cli;
use daemon_slayer::client::cli::ClientCliProvider;
use daemon_slayer::client::config::Level;
use daemon_slayer::client::config::windows::{ServiceAccess, Trustee, WindowsConfig};
use daemon_slayer::client::{self};
use daemon_slayer::config::cli::ConfigCliProvider;
use daemon_slayer::config::server::ConfigService;
use daemon_slayer::config::{AppConfig, ConfigDir};
use daemon_slayer::console::cli::ConsoleCliProvider;
use daemon_slayer::console::{self, Console, LogSource};
use daemon_slayer::core::BoxedError;
use daemon_slayer::error_handler::ErrorSink;
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::color_eyre::eyre;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{self, LoggerBuilder};
use daemon_slayer::process::cli::ProcessCliProvider;
use derive_more::AsRef;

#[derive(Debug, confique::Config, AsRef, Default, Clone)]
struct MyConfig {
    #[as_ref]
    #[config(nested)]
    client_config: client::config::UserConfig,
    #[as_ref]
    #[config(nested)]
    console_config: console::UserConfig,
    #[as_ref]
    #[config(nested)]
    logging_config: logging::UserConfig,
}

#[tokio::main]
pub async fn main() -> Result<(), ErrorSink> {
    let guard = daemon_slayer::logging::init();
    let result = run().await.map_err(|e| ErrorSink::new(eyre::eyre!(e)));
    drop(guard);
    result
}

async fn run() -> Result<(), BoxedError> {
    let app_config =
        AppConfig::<MyConfig>::builder(ConfigDir::ProjectDir(notifications::label())).build()?;

    let config = app_config.read_config().unwrap_or_default();
    let manager = client::builder(
        notifications::label(),
        current_exe()?
            .parent()
            .expect("Current exe should have a parent")
            .join("notification-server")
            .try_into()?,
    )
    .with_description("test service")
    .with_arg(&notifications::run_argument())
    .with_service_level(if cfg!(windows) {
        Level::System
    } else {
        Level::User
    })
    .with_windows_config(WindowsConfig::default().with_additional_access(
        Trustee::CurrentUser,
        ServiceAccess::Start | ServiceAccess::Stop | ServiceAccess::ChangeConfig,
    ))
    .with_user_config(config.clone())
    .build()
    .await?;

    let logger_builder = LoggerBuilder::new(notifications::label());

    let app_config_ = app_config.clone();
    let console = Console::new(manager.clone(), LogSource::Ipc)
        .await
        .with_config(app_config.clone())
        .with_configure_services(|context| {
            context.spawn(ConfigService::new(app_config_));
        });

    let mut cli = Cli::builder()
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ProcessCliProvider::new(manager.pid().await?))
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(
            ConfigCliProvider::new(app_config.clone()).with_config_watcher(manager.clone()),
        )
        .initialize()?;

    let (logger, _) = cli
        .take_provider::<LoggingCliProvider>()
        .get_logger_with_reload(app_config.clone())?;
    logger.init();

    cli.handle_input().await?;

    Ok(())
}
