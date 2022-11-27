use arc_swap::access::{DynAccess, Map};
use arc_swap::ArcSwap;
use confique::Config;
use daemon_slayer::cli::{ActionType, Cli};
use daemon_slayer::client::cli::ClientCliProvider;
use daemon_slayer::client::config::{ServiceAccess, UserConfig};
use daemon_slayer::client::{
    config::{Trustee, WindowsConfig},
    Level, Manager, ServiceManager,
};
use daemon_slayer::config::{AppConfig, ConfigFileType};
use daemon_slayer::console::cli::ConsoleCliProvider;
use daemon_slayer::console::Console;
use daemon_slayer::core::config::Configurable;
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
use daemon_slayer::signals::SignalHandlerTrait;
use daemon_slayer::signals::{Signal, SignalHandler};
use futures::{SinkExt, StreamExt};
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::trace::TraceLayer;
use tracing::metadata::LevelFilter;
use tracing::{error, info, warn};

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[derive(Debug, confique::Config, Default)]
struct MyConfig {
    #[config(nested)]
    client_config: UserConfig,
    #[config(default = "yes")]
    test: String,
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
        .with_user_config(Box::new(Map::new(config.clone(), |conf: &MyConfig| {
            &conf.client_config
        })))
        .build()?;

    let logger_builder = LoggerBuilder::new("daemon_slayer_combined").with_ipc_logger(true);
    let logging_provider = LoggingCliProvider::new(logger_builder);

    let health_check = IpcHealthCheck::new("daemon_slayer_combined");

    let mut console = Console::new(manager.clone());
    console.add_health_check(Box::new(health_check.clone()));
    let cli = Cli::builder()
        .with_default_client_commands()
        .with_default_server_commands()
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
    async fn new(context: &mut ServiceContext) -> Self {
        let (_, signal_store) = context.add_event_service(SignalHandler::all()).await;
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
