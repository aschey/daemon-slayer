use axum::routing::get;
use axum::Router;
use daemon_slayer::cli::Cli;
use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{cli::LoggingCliProvider, LoggerBuilder};
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, Handler, ServiceContext, Signal,
    SignalHandler,
};
use daemon_slayer::signals::SignalListener;
use futures::StreamExt;
use tracing::info;

#[tokio::main]
pub async fn main() {
    let run_argument = "-r".parse().unwrap();
    let logger_builder = LoggerBuilder::new(ServiceHandler::label());

    let mut cli = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::new(&run_argument))
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(LoggingCliProvider::new(logger_builder))
        .initialize()
        .unwrap();

    let logger_provider = cli.get_provider::<LoggingCliProvider>().unwrap();
    let (logger, _) = logger_provider.clone().get_logger().unwrap();
    logger.init();

    cli.handle_input().await.unwrap();
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    type InputData = ();
    type Error = BoxedError;

    async fn new(
        mut context: ServiceContext,
        _input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let signal_listener = SignalListener::all();
        let signal_store = signal_listener.get_event_store();
        context.add_service(signal_listener).await.unwrap();

        Ok(Self { signal_store })
    }

    fn label() -> Label {
        integration_tests::label()
    }

    async fn run_service<F: FnOnce() + Send>(mut self, on_started: F) -> Result<(), Self::Error> {
        info!("running service");

        let app = Router::new()
            .route("/test", get(test))
            .route("/env", get(env));

        on_started();
        info!("started");
        let mut signal_rx = self.signal_store.subscribe_events();
        axum::Server::bind(&integration_tests::address())
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
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
