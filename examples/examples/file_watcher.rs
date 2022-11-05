use daemon_slayer::client::cli::ClientCliProvider;
use daemon_slayer::client::config::{ServiceAccess, Trustee, WindowsConfig};
use daemon_slayer::client::{Level, Manager, ServiceManager};
use daemon_slayer::console::cli::ConsoleCliProvider;
use daemon_slayer::console::Console;
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::{self, ErrorHandler};
use daemon_slayer::file_watcher::{FileWatcher, FileWatcherBuilder};
use daemon_slayer::health_check::cli::HealthCheckCliProvider;
use daemon_slayer::health_check::IpcHealthCheck;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::signals::{Signal, SignalHandler, SignalHandlerBuilder};
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use daemon_slayer::cli::{ActionType, Cli};

use daemon_slayer::ipc_health_check;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, Handler, Service, ServiceContext,
};
use daemon_slayer::signals::SignalHandlerBuilderTrait;
use futures::{SinkExt, StreamExt};
use tower_http::trace::TraceLayer;
use tracing::metadata::LevelFilter;
use tracing::{error, info, warn};

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ServiceManager::builder("daemon_slayer_file_watcher")
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
        .build()?;

    let logger_builder = LoggerBuilder::new("daemon_slayer_file_watcher").with_ipc_logger(true);
    let logging_provider = LoggingCliProvider::new(logger_builder);

    let health_check = IpcHealthCheck::new("daemon_slayer_file_watcher");

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
        .build();

    let (logger, _guard) = logging_provider.get_logger();

    logger.init();
    cli.handle_input().await;

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
    file_watcher_store: BroadcastEventStore<Vec<PathBuf>>,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    async fn new(context: &mut ServiceContext) -> Self {
        let (_, signal_store) = context
            .add_event_service::<SignalHandler>(SignalHandlerBuilder::all())
            .await;
        context
            .add_service::<ipc_health_check::Server>(ipc_health_check::Builder::new(
                "daemon_slayer_file_watcher".to_owned(),
            ))
            .await;
        let (_, file_watcher_events) = context
            .add_event_service::<FileWatcher>(
                FileWatcherBuilder::default()
                    .with_watch_path(PathBuf::from("./assets/config.toml")),
            )
            .await;

        Self {
            signal_store,
            file_watcher_store: file_watcher_events,
        }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_file_watcher"
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");
        on_started();
        let mut signal_rx = self.signal_store.subscribe_events();
        let mut file_rx = self.file_watcher_store.subscribe_events();
        loop {
            tokio::select! {
                _ = signal_rx.next() => { return Ok(()); }
                files = file_rx.next() => {
                    info!("Files updated: {files:?}");
                }
            }
        }
    }
}
