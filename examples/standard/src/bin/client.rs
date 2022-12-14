use std::env::current_exe;

use daemon_slayer::{
    cli::Cli,
    client::{
        self,
        cli::ClientCliProvider,
        config::{
            windows::{ServiceAccess, Trustee, WindowsConfig},
            Level,
        },
    },
    config::{cli::ConfigCliProvider, server::ConfigService, AppConfig, ConfigDir},
    console::{self, cli::ConsoleCliProvider, Console},
    core::BoxedError,
    error_handler::{cli::ErrorHandlerCliProvider, ErrorSink},
    logging::{
        self, cli::LoggingCliProvider, tracing_subscriber::util::SubscriberInitExt, LoggerBuilder,
    },
    process::cli::ProcessCliProvider,
};
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
    let result = run().await.map_err(ErrorSink::from_error);
    drop(guard);
    result
}

async fn run() -> Result<(), BoxedError> {
    let app_config =
        AppConfig::<MyConfig>::builder(ConfigDir::ProjectDir(standard::label())).build()?;

    let config = app_config.read_config().unwrap_or_default();
    let manager = client::builder(
        standard::label(),
        current_exe()?
            .parent()
            .expect("Current exe should have a parent")
            .join("standard-server")
            .try_into()?,
    )
    .with_description("test service")
    .with_arg(&standard::run_argument())
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
    .build()?;

    let logger_builder = LoggerBuilder::new(standard::label()).with_config(app_config.clone());

    let app_config_ = app_config.clone();
    let console = Console::new(manager.clone())
        .with_config(app_config.clone())
        .with_configure_services(move |mut context| {
            let app_config = app_config_.clone();
            async move {
                context
                    .add_service(ConfigService::new(app_config))
                    .await
                    .unwrap();
            }
        });

    let mut cli = Cli::builder()
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ProcessCliProvider::new(manager.info()?.pid))
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(
            ConfigCliProvider::new(app_config.clone()).with_config_watcher(manager.clone()),
        )
        .initialize()?;

    let (logger, _) = cli
        .get_provider::<LoggingCliProvider>()
        .unwrap()
        .clone()
        .get_logger()?;
    logger.init();

    cli.handle_input().await?;

    Ok(())
}
