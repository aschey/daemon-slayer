use std::error::Error;
use std::time::Duration;

use arc_swap::access::{DynAccess, Map};
use arc_swap::ArcSwap;
use confique::Config;
use daemon_slayer::cli::{ActionType, Cli};
use daemon_slayer::client::cli::ClientCliProvider;
use daemon_slayer::client::config::ServiceAccess;
use daemon_slayer::client::{
    self,
    config::{Trustee, WindowsConfig},
    Level, Manager, ServiceManager,
};
use daemon_slayer::config::{self, AppConfig, ConfigFileType};
use daemon_slayer::console::cli::ConsoleCliProvider;
use daemon_slayer::console::{self, Console};
use daemon_slayer::core::config::{Accessor, Configurable};
use daemon_slayer::core::server::Toplevel;
use daemon_slayer::core::App;
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::health_check::cli::HealthCheckCliProvider;
use daemon_slayer::health_check::IpcHealthCheck;
use daemon_slayer::ipc_health_check;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, Handler, Service, ServiceContext,
};
use daemon_slayer::server::{Signal, SignalHandler};
use daemon_slayer::signals::SignalListener;
use futures::StreamExt;
use tracing::info;

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[derive(Debug, confique::Config, Default, Clone)]
struct MyConfig {
    #[config(nested)]
    client_config: client::config::UserConfig,
    #[config(nested)]
    console_config: console::UserConfig,
    #[config(default = "yes")]
    test: String,
}

impl AsRef<client::config::UserConfig> for MyConfig {
    fn as_ref(&self) -> &client::config::UserConfig {
        &self.client_config
    }
}

impl AsRef<console::UserConfig> for MyConfig {
    fn as_ref(&self) -> &console::UserConfig {
        &self.console_config
    }
}

#[tokio::main]
pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let app_config = AppConfig::<MyConfig>::new(
        App {
            application: "combined".to_owned(),
            organization: "daemonslayer".to_owned(),
            qualifier: "com".to_owned(),
        },
        ConfigFileType::Toml,
    );

    app_config.create_config_template();
    let config = app_config.read_config();
    let manager = ServiceManager::builder("daemon_slayer_combined")
        .with_description("test service")
        .with_args(["run"])
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

    let logger_builder = LoggerBuilder::new("daemon_slayer_combined");
    let logging_provider = LoggingCliProvider::new(logger_builder);

    let health_check = IpcHealthCheck::new("daemon_slayer_combined");

    let console = Console::new(manager.clone())
        .with_health_check(Box::new(health_check.clone()))
        .with_config(app_config.clone());

    let cli = Cli::builder()
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ServerCliProvider::<ServiceHandler>::default())
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(HealthCheckCliProvider::new(health_check))
        .with_provider(logging_provider.clone())
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(daemon_slayer::config::cli::ConfigCliProvider::new(
            app_config,
            manager.clone(),
        ))
        .initialize();

    let (logger, _guard) = logging_provider.get_logger();
    logger.init();

    cli.handle_input().await;

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    async fn new(mut context: ServiceContext) -> Self {
        let (_, signal_store) = context.add_event_service(SignalListener::all()).await;
        context
            .add_service(ipc_health_check::Server::new(
                "daemon_slayer_combined".to_owned(),
            ))
            .await;

        Self { signal_store }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_combined"
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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
