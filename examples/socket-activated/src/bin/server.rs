use axum::extract::Path;
use axum::routing::get;
use axum::Router;
use confique::Config;
use daemon_slayer::build_info::cli::BuildInfoCliProvider;
use daemon_slayer::build_info::vergen_pretty::{self, Style};
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
use daemon_slayer::server::socket_activation::{ActivationSockets, SocketResult};
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, ServiceContext, Signal, SignalHandler,
};
use daemon_slayer::signals::SignalListener;
use derive_more::AsRef;
use tower_http::trace::TraceLayer;
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
        AppConfig::<MyConfig>::builder(ConfigDir::ProjectDir(socket_activated::label())).build()?;

    let logger_builder =
        LoggerBuilder::new(ServiceHandler::label()).with_config(app_config.clone());
    let pretty = vergen_pretty::PrettyBuilder::default()
        .env(vergen_pretty::vergen_pretty_env!())
        .category(false)
        .key_style(Style::new().bold().cyan())
        .value_style(Style::new())
        .build()?;
    let mut cli = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::new(
            &socket_activated::run_argument(),
        ))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(ConfigCliProvider::new(app_config.clone()))
        .with_provider(BuildInfoCliProvider::new(pretty))
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
        socket_activated::label()
    }

    async fn new(
        mut context: ServiceContext,
        input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let input_data = input_data.unwrap();
        let signal_listener = SignalListener::all();
        let signal_store = signal_listener.get_event_store();
        context.add_service(signal_listener);

        let config_service = ConfigService::new(input_data.config);
        let file_events = config_service.get_event_store();
        context.add_service(config_service);
        context.add_service(LoggingUpdateService::new(
            input_data.reload_handle,
            file_events,
        ));

        Ok(Self { signal_store })
    }

    async fn run_service<F: FnOnce() + Send>(mut self, notify_ready: F) -> Result<(), Self::Error> {
        info!("running service");
        notify_ready();

        let mut sockets = ActivationSockets::get(socket_activated::sockets());
        let socket = sockets.next().await.unwrap();
        let SocketResult::Tcp(listener) = socket else {
            panic!()
        };

        let app = Router::new()
            .route("/hello/:name", get(greeter))
            // .route("/task", get(start_task))
            .route("/health", get(health))
            .layer(TraceLayer::new_for_http());

        let mut signals = self.signal_store.subscribe_events();
        axum::Server::from_tcp(listener.into_std().unwrap())
            .unwrap()
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                signals.next().await;
                info!("Got shutdown request");
            })
            .await?;
        info!("Server terminated");

        Ok(())
    }
}

async fn greeter(Path(name): Path<String>) -> String {
    format!("Hello {name}")
}

async fn health() -> &'static str {
    "healthy"
}
