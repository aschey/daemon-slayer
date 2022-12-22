use std::error::Error;
use std::process::{self, ExitCode};
use std::time::Duration;

use arc_swap::access::{DynAccess, Map};
use arc_swap::ArcSwap;
use confique::Config;
use daemon_slayer::cli::{ActionType, Cli};
use daemon_slayer::client::cli::ClientCliProvider;
use daemon_slayer::client::configuration::Level;
use daemon_slayer::client::{
    self,
    configuration::windows::{ServiceAccess, Trustee, WindowsConfiguration},
    Manager,
};
use daemon_slayer::config::server::ConfigService;
use daemon_slayer::config::{self, AppConfig, ConfigFileType};
use daemon_slayer::console::cli::ConsoleCliProvider;
use daemon_slayer::console::{self, Console};
use daemon_slayer::core::config::Accessor;
use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::color_eyre::{eyre, Report};
use daemon_slayer::error_handler::ErrorSink;
use daemon_slayer::health_check::cli::HealthCheckCliProvider;
use daemon_slayer::health_check::IpcHealthCheck;
use daemon_slayer::logging;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::server::LoggingUpdateService;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, Handler, Service, ServiceContext,
};
use daemon_slayer::server::{Signal, SignalHandler};
use daemon_slayer::signals::SignalListener;
use futures::StreamExt;
use tracing::{error, info};

pub fn main() -> Result<(), ErrorSink> {
    daemon_slayer::logging::init_local_time();
    run_async().map_err(ErrorSink::from_error)
}

#[derive(Debug, confique::Config, Default, Clone)]
struct MyConfig {
    #[config(nested)]
    client_config: client::configuration::UserConfiguration,
    #[config(nested)]
    console_config: console::UserConfig,
    #[config(nested)]
    logging_config: logging::UserConfig,
    #[config(default = "yes")]
    test: String,
}

impl AsRef<client::configuration::UserConfiguration> for MyConfig {
    fn as_ref(&self) -> &client::configuration::UserConfiguration {
        &self.client_config
    }
}

impl AsRef<console::UserConfig> for MyConfig {
    fn as_ref(&self) -> &console::UserConfig {
        &self.console_config
    }
}

impl AsRef<logging::UserConfig> for MyConfig {
    fn as_ref(&self) -> &logging::UserConfig {
        &self.logging_config
    }
}

#[derive(Clone)]
pub struct AppData {
    config: AppConfig<MyConfig>,
    logger_guard: LoggerGuard,
}

#[tokio::main]
pub async fn run_async() -> Result<(), BoxedError> {
    let app_config = AppConfig::<MyConfig>::new(ServiceHandler::label(), ConfigFileType::Toml);

    app_config.create_config_template();
    let config = app_config.read_config();
    let manager = client::builder(ServiceHandler::label())
        .with_description("test service")
        .with_args(["run"])
        .with_service_level(if cfg!(windows) {
            Level::System
        } else {
            Level::User
        })
        .with_windows_configuration(WindowsConfiguration::default().with_additional_access(
            Trustee::CurrentUser,
            ServiceAccess::Start | ServiceAccess::Stop | ServiceAccess::ChangeConfig,
        ))
        .with_user_configuration(config.clone())
        .build()?;

    let logger_builder =
        LoggerBuilder::new(ServiceHandler::label()).with_config(app_config.clone());

    let health_check = IpcHealthCheck::new(ServiceHandler::label().application);
    let app_config_ = app_config.clone();
    let console = Console::new(manager.clone())
        .with_health_check(Box::new(health_check.clone()))
        .with_config(app_config.clone())
        .with_configure_services(move |mut context| {
            let app_config = app_config_.clone();
            async move {
                context.add_service(ConfigService::new(app_config)).await;
            }
        });

    let mut cli = Cli::builder()
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ServerCliProvider::<ServiceHandler>::default())
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(HealthCheckCliProvider::new(health_check))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(daemon_slayer::config::cli::ConfigCliProvider::new(
            app_config.clone(),
            manager,
        ))
        .initialize()?;

    let (logger, guard) = cli
        .get_provider::<LoggingCliProvider>()
        .unwrap()
        .clone()
        .get_logger();
    logger.init();

    cli.get_provider::<ServerCliProvider<ServiceHandler>>()
        .unwrap()
        .set_input_data(AppData {
            config: app_config,
            logger_guard: guard,
        });

    cli.handle_input().await?;

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = AppData;

    fn label() -> Label {
        "com.example.daemonslayercombined".parse().unwrap()
    }

    async fn new(mut context: ServiceContext, input_data: Option<Self::InputData>) -> Self {
        let input_data = input_data.unwrap();
        let (_, signal_store) = context.add_event_service(SignalListener::all()).await;
        context
            .add_service(daemon_slayer::ipc::health_check::Server::new(
                Self::label().application,
            ))
            .await;
        let (_, file_events) = context
            .add_event_service(ConfigService::new(input_data.config))
            .await;
        context
            .add_service(LoggingUpdateService::new(
                input_data.logger_guard,
                file_events,
            ))
            .await;

        Self { signal_store }
    }

    async fn run_service<F: FnOnce() + Send>(mut self, on_started: F) -> Result<(), Self::Error> {
        info!("running service");
        on_started();
        let mut signal_rx = self.signal_store.subscribe_events();
        loop {
            match tokio::time::timeout(Duration::from_secs(1), signal_rx.next()).await {
                Ok(_) => {
                    info!("stopping service");
                    return Ok(());
                }
                Err(_) => {
                    info!("var {:?}", std::env::var("test"));
                }
            }
        }
    }
}
