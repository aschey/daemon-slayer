use std::time::{Duration, Instant};

use clap::{FromArgMatches as _, Parser, Subcommand as _};
use daemon_slayer::cli::{Cli, InputState};
use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::color_eyre::eyre;
use daemon_slayer::error_handler::ErrorSink;
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::LoggerBuilder;
use daemon_slayer::server::cli::ServerCliProvider;
use daemon_slayer::server::futures::StreamExt;
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, ServiceContext, Signal, SignalHandler,
};
use daemon_slayer::signals::SignalListener;
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
    let result = run().await.map_err(|e| ErrorSink::new(eyre::eyre!(e)));
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

impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = ();

    fn label() -> Label {
        "com.example.daemon_slayer_custom_command"
            .parse()
            .expect("Should parse the label")
    }

    async fn new(context: ServiceContext, _: Option<Self::InputData>) -> Result<Self, Self::Error> {
        let signal_listener = SignalListener::termination();
        let signal_store = signal_listener.get_event_store();
        context.spawn(signal_listener);

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
                    )
                }
            }
        }
    }
}
