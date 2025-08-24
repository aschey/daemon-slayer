use axum::Router;
use axum::routing::get;
use daemon_slayer::cli::Cli;
use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::logging::LoggerBuilder;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::time::format_description::well_known::Rfc3339;
use daemon_slayer::logging::tracing_subscriber::fmt::time::OffsetTime;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::server::cli::ServerCliProvider;
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, ServiceContext, Signal, SignalHandler,
};
use daemon_slayer::signals::SignalListener;
use futures::StreamExt;
use tracing::info;

fn main() {
    let local_time = OffsetTime::local_rfc_3339().unwrap();
    async_main(local_time);
}

#[tokio::main]
async fn async_main(local_time: OffsetTime<Rfc3339>) {
    let logger_builder = LoggerBuilder::new(ServiceHandler::label(), local_time);

    let mut cli = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::new(
            &integration_tests::service_arg(),
        ))
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(LoggingCliProvider::new(logger_builder))
        .initialize()
        .unwrap();

    let logger_provider = cli.take_provider::<LoggingCliProvider<Rfc3339>>();
    let logger = logger_provider.get_logger().unwrap();
    logger.init();

    cli.handle_input().await.unwrap();
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

impl Handler for ServiceHandler {
    type InputData = ();
    type Error = BoxedError;

    async fn new(
        context: ServiceContext,
        _input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let signal_listener = SignalListener::all();
        let signal_store = signal_listener.get_event_store();
        context.spawn(signal_listener);

        Ok(Self { signal_store })
    }

    fn label() -> Label {
        integration_tests::label()
    }

    async fn run_service<F: FnOnce() + Send>(self, notify_ready: F) -> Result<(), Self::Error> {
        info!("running service");

        let app = Router::new()
            .route("/test", get(test))
            .route("/env", get(env));

        notify_ready();
        info!("started");
        let listener = tokio::net::TcpListener::bind(integration_tests::address())
            .await
            .unwrap();
        let mut signal_rx = self.signal_store.subscribe_events();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = signal_rx.next().await;
            })
            .await
            .unwrap();
        Ok(())
    }
}

async fn test() -> &'static str {
    "test"
}

async fn env() -> String {
    std::env::var("DAEMON_SLAYER_TEST_ENV").unwrap_or_default()
}
