use clap::{FromArgMatches as _, Parser, Subcommand as _};
use daemon_slayer::{
    cli::{Cli, InputState},
    core::{BoxedError, Label},
    error_handler::{cli::ErrorHandlerCliProvider, ErrorSink},
    logging::{
        cli::LoggingCliProvider, tracing_subscriber::util::SubscriberInitExt, LoggerBuilder,
    },
    server::{
        cli::ServerCliProvider, futures::StreamExt, BroadcastEventStore, EventStore, Handler,
        ServiceContext, Signal, SignalHandler,
    },
    signals::SignalListener,
};
use std::time::{Duration, Instant};
use tracing::info;

#[derive(Parser, Debug)]
enum Subcommands {
    Derived {
        #[arg(short, long)]
        derived_flag: bool,
    },
}

#[tokio::main]
pub async fn main() -> Result<(), ErrorSink> {
    let guard = daemon_slayer::logging::init();
    let result = run().await.map_err(ErrorSink::from_error);
    drop(guard);
    result
}

async fn run() -> Result<(), BoxedError> {
    let logger_builder = LoggerBuilder::new(ServiceHandler::label());

    let clap_cmd = Subcommands::augment_subcommands(clap::Command::default());
    let mut cli = Cli::builder()
        .with_base_command(clap_cmd)
        .with_provider(ServerCliProvider::<ServiceHandler>::new(
            &"run".parse().expect("failed to parse the run argument"),
        ))
        .with_provider(LoggingCliProvider::new(logger_builder))
        .with_provider(ErrorHandlerCliProvider::default())
        .initialize()?;

    let (logger, _) = cli.take_provider::<LoggingCliProvider>().get_logger()?;
    logger.init();

    if let (InputState::Unhandled, matches) = cli.handle_input().await? {
        if let Ok(cmd) = Subcommands::from_arg_matches(&matches) {
            println!("Derived: {cmd:?}");
        }
    }

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

#[daemon_slayer::core::async_trait]
impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = ();

    fn label() -> Label {
        "com.example.daemon_slayer_custom_command"
            .parse()
            .expect("Should parse the label")
    }

    async fn new(
        mut context: ServiceContext,
        _: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let signal_listener = SignalListener::all();
        let signal_store = signal_listener.get_event_store();
        context.add_service(signal_listener).await?;

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
                    )
                }
            }
        }
    }
}
