use daemon_slayer::client::cli::ClientCliProvider;
use daemon_slayer::client::{Level, Manager, ServiceManager};
use daemon_slayer::console::cli::ConsoleCliProvider;
use daemon_slayer::console::Console;
use daemon_slayer::error_handler::{self, ErrorHandler};
use daemon_slayer::health_check::cli::HealthCheckCliProvider;
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

use daemon_slayer::cli::{clap, ActionType, Cli, InputState};

use daemon_slayer::ipc_health_check;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, Handler, Receiver, Service,
    ServiceContext,
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
    let manager = ServiceManager::builder("daemon_slayer_custom_command")
        .with_description("test service")
        .with_args(["run"])
        .with_service_level(if cfg!(windows) {
            Level::System
        } else {
            Level::User
        })
        .build()?;

    let health_check = IpcHealthCheck::new("daemon_slayer_custom_command");

    let mut console = Console::new(manager.clone());
    console.add_health_check(Box::new(health_check.clone()));

    let base_command = clap::Command::default()
        .subcommand(clap::Command::new("custom").about("custom subcommand"))
        .arg(
            clap::Arg::new("custom_arg")
                .short('c')
                .long("custom")
                .help("custom arg"),
        );

    let (cli, command) = Cli::builder()
        .with_base_command(base_command)
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ServerCliProvider::<ServiceHandler>::default())
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(HealthCheckCliProvider::new(health_check))
        .build();

    let matches = command.get_matches();

    let (logger, _guard, error_handler) = match cli.action_type(&matches) {
        ActionType::Server => {
            let (logger, guard) = LoggerBuilder::for_server("daemon_slayer_custom_command")
                .with_ipc_logger(true)
                .build()?;
            (logger, guard, ErrorHandler::for_server())
        }
        ActionType::Client => {
            let (logger, guard) = LoggerBuilder::for_client("daemon_slayer_custom_command")
                .with_ipc_logger(true)
                .build()?;
            (logger, guard, ErrorHandler::for_client())
        }
        ActionType::Unknown => {
            let (logger, guard) = LoggerBuilder::new("daemon_slayer_custom_command")
                .with_ipc_logger(true)
                .build()?;
            (logger, guard, ErrorHandler::default())
        }
    };
    logger.init();
    error_handler.install().unwrap();

    if cli.handle_input(&matches).await == InputState::Unhandled {
        if let Some(("custom", _)) = matches.subcommand() {
            println!("matched custom command");
        }
        if let Some(arg) = matches.get_one::<String>("custom_arg") {
            println!("custom arg is {}", arg);
        }
    }

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
                "daemon_slayer_custom_command".to_owned(),
            ))
            .await;

        Self { signal_store }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_custom_command"
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");
        on_started();
        let mut signal_rx = self.signal_store.subscribe_events();
        loop {
            match tokio::time::timeout(Duration::from_secs(1), signal_rx.recv()).await {
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

// let base_command = clap::Command::default()
// .subcommand(clap::Command::new("custom").about("custom subcommand"))
// .arg(
//     clap::Arg::new("custom_arg")
//         .short('c')
//         .long("custom")
//         .help("custom arg"),
// );

// if let InputState::Unhandled(matches) = cli.handle_input().await? {
//     if let Some(("custom", _)) = matches.subcommand() {
//         println!("matched custom command");
//     }
//     if let Some(arg) = matches.get_one::<String>("custom_arg") {
//         println!("custom arg is {}", arg);
//     }
// }
