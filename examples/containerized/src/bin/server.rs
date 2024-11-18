use std::time::{Duration, Instant};

use confique::Config;
use daemon_slayer::cli::Cli;
use daemon_slayer::config::cli::ConfigCliProvider;
use daemon_slayer::config::server::ConfigService;
use daemon_slayer::config::{AppConfig, ConfigDir};
use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::color_eyre::eyre;
use daemon_slayer::error_handler::ErrorSink;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::server::LoggingUpdateService;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{self, LoggerBuilder, ReloadHandle};
use daemon_slayer::server::cli::ServerCliProvider;
use daemon_slayer::server::futures::StreamExt;
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, ServiceContext, Signal, SignalHandler,
};
use daemon_slayer::signals::SignalListener;
use derive_more::AsRef;
use tracing::info;

#[derive(Debug, Config, AsRef, Default, Clone)]
struct MyConfig {
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

#[derive(Clone)]
pub struct AppData {
    config: AppConfig<MyConfig>,
    reload_handle: ReloadHandle,
}

async fn run() -> Result<(), BoxedError> {
    let app_config =
        AppConfig::<MyConfig>::builder(ConfigDir::ProjectDir(containerized::label())).build()?;

    let logger_builder = LoggerBuilder::new(ServiceHandler::label());

    let mut cli = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::new(
            &containerized::run_argument(),
        ))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(ConfigCliProvider::new(app_config.clone()))
        .initialize()?;

    let (logger, reload_handle) = cli
        .take_provider::<LoggingCliProvider>()
        .get_logger_with_reload(app_config.clone())?;

    logger.init();

    cli.get_provider::<ServerCliProvider<ServiceHandler>>()
        .set_input_data(AppData {
            config: app_config,
            reload_handle: reload_handle.clone(),
        });

    cli.handle_input().await?;

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = AppData;

    fn label() -> Label {
        containerized::label()
    }

    async fn new(
        context: ServiceContext,
        input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let input_data = input_data.unwrap();
        let signal_listener = SignalListener::termination();
        let signal_store = signal_listener.get_event_store();
        context.spawn(signal_listener);

        let config_service = ConfigService::new(input_data.config);
        let file_events = config_service.get_event_store();
        context.spawn(config_service);
        context.spawn(LoggingUpdateService::new(
            input_data.reload_handle,
            file_events,
        ));

        Ok(Self { signal_store })
    }

    async fn run_service<F: FnOnce() + Send>(self, notify_ready: F) -> Result<(), Self::Error> {
        info!("running service");
        notify_ready();

        let mut signal_rx = self.signal_store.subscribe_events();
        let start_time = Instant::now();
        loop {
            match tokio::time::timeout(Duration::from_secs(1), signal_rx.next()).await {
                Ok(_) => {
                    info!("stopping service");
                    return Ok(());
                }
                Err(_) => {
                    info!(
                        "Run time: {} seconds",
                        Instant::now().duration_since(start_time).as_secs()
                    );
                }
            }
        }
    }
}
