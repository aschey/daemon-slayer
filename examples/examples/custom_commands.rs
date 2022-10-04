use std::env::args;
use std::error::Error;
use std::time::{Duration, Instant};

use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{clap, Action, ActionType, CliAsync, InputState};
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::server::{EventHandlerAsync, HandlerAsync, ServiceAsync};

use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use daemon_slayer::client::Level;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use futures::{SinkExt, StreamExt};
use tracing::info;

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ServiceManager::builder(ServiceHandler::get_service_name())
        .with_description("test service")
        .with_service_level(if cfg!(windows) {
            Level::System
        } else {
            Level::User
        })
        .with_autostart(true)
        .with_args(["run"])
        .build()
        .unwrap();

    let base_command = clap::Command::default()
        .subcommand(clap::Command::new("custom").about("custom subcommand"))
        .arg(
            clap::Arg::new("custom_arg")
                .short('c')
                .long("custom")
                .help("custom arg"),
        );
    let cli = CliAsync::builder_for_all(manager, ServiceHandler::new())
        .with_base_command(base_command)
        .build();

    let (logger, _guard) = cli
        .configure_logger()
        .with_ipc_logger(true)
        .build()
        .unwrap();

    logger.init();

    cli.configure_error_handler().install()?;

    if let InputState::Unhandled(matches) = cli.handle_input().await? {
        if let Some(("custom", _)) = matches.subcommand() {
            println!("matched custom command");
        }
        if let Some(arg) = matches.get_one::<String>("custom_arg") {
            println!("custom arg is {}", arg);
        }
    }
    Ok(())
}

#[derive(daemon_slayer::server::ServiceAsync)]
pub struct ServiceHandler {
    tx: futures::channel::mpsc::Sender<()>,
    rx: futures::channel::mpsc::Receiver<()>,
}

#[async_trait::async_trait]
impl HandlerAsync for ServiceHandler {
    fn new() -> Self {
        let (tx, rx) = futures::channel::mpsc::channel(32);
        Self { tx, rx }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_custom_command"
    }

    fn get_event_handler(&mut self) -> EventHandlerAsync {
        let tx = self.tx.clone();
        Box::new(move |event| {
            let mut tx = tx.clone();
            Box::pin(async move {
                info!("stopping");
                let _ = tx.send(()).await;
                Ok(())
            })
        })
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");
        on_started();
        loop {
            match tokio::time::timeout(Duration::from_secs(1), self.rx.next()).await {
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
