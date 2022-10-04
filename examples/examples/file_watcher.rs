use std::env::args;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{Action, ActionType, CliAsync, Command};
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::server::{EventHandlerAsync, HandlerAsync, ServiceAsync};

use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use daemon_slayer::client::Level;
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
        .with_autostart(false)
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = CliAsync::for_all(manager, ServiceHandler::new());

    let (logger, _guard) = cli
        .configure_logger()
        .with_ipc_logger(true)
        .build()
        .unwrap();

    logger.init();

    cli.configure_error_handler().install()?;
    cli.handle_input().await?;
    Ok(())
}

#[derive(daemon_slayer::server::ServiceAsync)]
pub struct ServiceHandler {
    tx: futures::channel::mpsc::Sender<()>,
    rx: futures::channel::mpsc::Receiver<()>,
    config_paths: Vec<PathBuf>,
}

#[async_trait::async_trait]
impl HandlerAsync for ServiceHandler {
    fn new() -> Self {
        let (tx, rx) = futures::channel::mpsc::channel(32);
        Self {
            tx,
            rx,
            config_paths: vec![PathBuf::from(
                "/home/aschey/code/daemon-slayer/examples/assets/config.toml",
            )],
        }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_async_file_watcher"
    }

    fn get_watch_paths(&self) -> Vec<PathBuf> {
        self.config_paths.clone()
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
