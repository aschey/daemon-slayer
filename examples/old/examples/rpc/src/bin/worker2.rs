use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::ipc::rpc::{self, RpcService};
use daemon_slayer::ipc::Codec;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::signals::{Signal, SignalListener};
use ipc::{get_rpc_service, Message, Topic};
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tarpc::context;

use daemon_slayer::cli::Cli;
use daemon_slayer::ipc::pubsub::{SubscriberClient, SubscriberServer};
use daemon_slayer::ipc_health_check;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::BackgroundService;
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, Handler, Service, ServiceContext,
};
use daemon_slayer::signals::SignalListenerTrait;
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
    subscriber: SubscriberClient<Topic, Message>,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    async fn new(context: &mut ServiceContext) -> Self {
        let (_, signal_store) = context
            .add_event_service::<SignalHandler>(SignalHandler::all())
            .await;
        let subscriber = context
            .spawn(SubscriberServer::<Topic, Message>::new(
                "daemon_slayer_ipc",
                Codec::Bincode,
            ))
            .await;

        Self {
            signal_store,
            subscriber,
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
        on_started();
        let mut topic1 = self.subscriber.subscribe([Topic::Topic1]).await;
        let mut topic2 = self.subscriber.subscribe([Topic::Topic2]).await;
        let mut signal_rx = self.signal_store.subscribe_events();
        let mut rpc_service = get_rpc_service();
        let rpc_client = rpc_service.get_client().await;
        loop {
            tokio::select! {
                _ = signal_rx.next() => {
                    info!("stopping service");
                    return Ok(());
                },
                msg = topic1.recv() => {
                    info!("Got topic1 message: {msg:?}");
                    rpc_client.ping(context::current()).await.unwrap();
                }
                msg = topic2.recv() => {
                    info!("Got topic2 message: {msg:?}");
                }
            }
        }
    }
}
