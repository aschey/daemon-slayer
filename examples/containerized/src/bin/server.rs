use confique::Config;
use daemon_slayer::{
    cli::Cli,
    config::{cli::ConfigCliProvider, server::ConfigService, AppConfig, ConfigDir},
    core::{BoxedError, Label},
    error_handler::{cli::ErrorHandlerCliProvider, ErrorSink},
    logging::{
        self, cli::LoggingCliProvider, server::LoggingUpdateService,
        tracing_subscriber::util::SubscriberInitExt, LoggerBuilder, ReloadHandle,
    },
    server::{
        cli::ServerCliProvider, futures::StreamExt, BroadcastEventStore, EventStore, Handler,
        ServiceContext, Signal, SignalHandler,
    },
    signals::SignalListener,
};
use derive_more::AsRef;
use std::time::{Duration, Instant};
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
    let result = run().await.map_err(ErrorSink::from_error);
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

    let logger_builder =
        LoggerBuilder::new(ServiceHandler::label()).with_config(app_config.clone());

    let mut cli = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::new(
            &containerized::run_argument(),
        ))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::new(containerized::label()))
        .with_provider(ConfigCliProvider::new(app_config.clone()))
        .initialize()?;

    let (logger, reload_handle) = cli.take_provider::<LoggingCliProvider>().get_logger()?;

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

#[daemon_slayer::core::async_trait]
impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = AppData;

    fn label() -> Label {
        containerized::label()
    }

    async fn new(
        mut context: ServiceContext,
        input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let input_data = input_data.unwrap();
        let signal_listener = SignalListener::all();
        let signal_store = signal_listener.get_event_store();
        context.add_service(signal_listener).await?;

        let config_service = ConfigService::new(input_data.config);
        let file_events = config_service.get_event_store();
        context.add_service(config_service).await?;
        context
            .add_service(LoggingUpdateService::new(
                input_data.reload_handle,
                file_events,
            ))
            .await?;

        Ok(Self { signal_store })
    }

    async fn run_service<F: FnOnce() + Send>(mut self, notify_ready: F) -> Result<(), Self::Error> {
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
