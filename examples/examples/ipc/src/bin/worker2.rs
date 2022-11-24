use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::ipc::rpc::{self, RpcService};
use daemon_slayer::ipc::Codec;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::signals::{Signal, SignalHandler};
use ipc::{IpcRequest, IpcResponse};
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tarpc::context;

use daemon_slayer::cli::Cli;
use daemon_slayer::ipc::{
    pubsub::{SubscriberClient, SubscriberServer},
    IpcClient,
};
use daemon_slayer::ipc_health_check;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::BackgroundService;
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, Handler, Service, ServiceContext,
};
use daemon_slayer::signals::SignalHandlerTrait;
use futures::{SinkExt, StreamExt};
use tracing::metadata::LevelFilter;
use tracing::{error, info, warn};

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let logger_builder = LoggerBuilder::new("daemon_slayer_ipc_worker2")
        .with_ipc_logger(true)
        .with_env_filter_directive("tarpc=warn".parse().unwrap());

    let logging_provider = LoggingCliProvider::new(logger_builder);

    let cli = Cli::builder()
        .with_default_server_commands()
        .with_provider(ServerCliProvider::<ServiceHandler>::default())
        .with_provider(logging_provider.clone())
        .with_provider(ErrorHandlerCliProvider::default())
        .initialize();

    let (logger, _guard) = logging_provider.get_logger();
    logger.init();

    cli.handle_input().await;

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
    ipc_client: IpcClient<IpcRequest, IpcResponse>,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    async fn new(context: &mut ServiceContext) -> Self {
        let (_, signal_store) = context
            .add_event_service::<SignalHandler>(SignalHandler::all())
            .await;
        let ipc_client =
            IpcClient::<IpcRequest, IpcResponse>::new("daemon_slayer_ipc", Codec::Json);

        Self {
            signal_store,
            ipc_client,
        }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_ipc_worker2"
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");
        let mut signal_rx = self.signal_store.subscribe_events();
        on_started();
        loop {
            match tokio::time::timeout(Duration::from_secs(1), signal_rx.next()).await {
                Ok(_) => {
                    info!("stopping service");
                    return Ok(());
                }
                Err(_) => {
                    let res = self
                        .ipc_client
                        .send(IpcRequest {
                            name: "joe".to_owned(),
                        })
                        .await;
                    info!("got response {res:?}");
                }
            }
        }
    }
}
