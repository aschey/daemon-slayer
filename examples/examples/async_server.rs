use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::health_check::IpcHealthCheck;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::signals::{Signal, SignalHandler, SignalHandlerBuilder};
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use daemon_slayer::cli::Cli;

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
    let (logger, _guard) = LoggerBuilder::for_server("daemon_slayer_async_server")
        .with_ipc_logger(true)
        .build()?;
    ErrorHandler::for_server().install()?;

    logger.init();
    let (cli, command) = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::default())
        .build();
    let matches = command.get_matches();
    cli.handle_input(&matches).await;

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    async fn new(context: &mut ServiceContext) -> Self {
        let (_, signal_store) = context
            .add_event_service::<SignalHandler>(SignalHandlerBuilder::all())
            .await;
        context
            .add_service::<ipc_health_check::Server>(ipc_health_check::Builder::new(
                "daemon_slayer_async_server".to_owned(),
            ))
            .await;

        Self { signal_store }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_async_server"
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
                    info!("Current time: {:?}", Instant::now());
                }
            }
        }
    }
}
