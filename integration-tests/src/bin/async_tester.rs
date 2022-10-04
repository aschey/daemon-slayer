use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use axum::extract::Path;
use axum::routing::get;
use axum::Router;
use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{Action, ActionType, CliAsync, Command, ServiceCommand};
use daemon_slayer::server::{Event, EventHandlerAsync, HandlerAsync, ServiceAsync};

use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use daemon_slayer::client::Level;
use futures::{SinkExt, StreamExt};
use serde_derive::Deserialize;
use tracing::info;

use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
pub async fn main() {
    let mut manager_builder = ServiceManager::builder(ServiceHandler::get_service_name())
        .with_description("test service")
        .with_args(["run"]);

    if let Ok(config_file) = std::env::var("CONFIG_FILE") {
        manager_builder = manager_builder.with_env_var("CONFIG_FILE", config_file);
    }

    let manager = manager_builder.build().unwrap();

    let cli = CliAsync::for_all(manager, ServiceHandler::new());

    let (logger, _guard) = cli
        .configure_logger()
        .with_ipc_logger(true)
        .build()
        .unwrap();
    logger.init();
    cli.configure_error_handler().install().unwrap();

    cli.handle_input().await.unwrap();
}

#[derive(daemon_slayer::server::ServiceAsync)]
pub struct ServiceHandler {
    tx: futures::channel::mpsc::Sender<()>,
    rx: futures::channel::mpsc::Receiver<()>,
}

static CONFIG: RwLock<Config> = RwLock::new(Config { test: false });

#[derive(Deserialize, Default)]
struct Config {
    test: bool,
}

#[async_trait::async_trait]
impl HandlerAsync for ServiceHandler {
    fn new() -> Self {
        let (tx, rx) = futures::channel::mpsc::channel(32);

        Self { tx, rx }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_test_service_async"
    }

    fn get_watch_paths(&self) -> Vec<PathBuf> {
        match std::env::var("CONFIG_FILE") {
            Ok(config_file) => {
                let abs_path = PathBuf::from(config_file);
                vec![abs_path]
            }
            Err(_) => {
                vec![]
            }
        }
    }

    fn get_event_handler(&mut self) -> EventHandlerAsync {
        let tx = self.tx.clone();
        Box::new(move |event| {
            let mut tx = tx.clone();
            Box::pin(async move {
                match event {
                    Event::SignalReceived(_) => {
                        info!("stopping");
                        tx.send(()).await?;
                    }
                    Event::FileChanged(files) => {
                        info!("reloading");
                        if let Some(file) = files.get(0) {
                            let contents = std::fs::read_to_string(file).unwrap();
                            (*CONFIG.write().unwrap()) =
                                toml::from_str::<Config>(&contents).unwrap();
                        }
                    }
                }

                Ok(())
            })
        })
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");

        let app = Router::new()
            .route("/config", get(config))
            .route("/health", get(health));
        let addr = SocketAddr::from(([127, 0, 0, 1], 3002));

        on_started();
        info!("started");
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                let _ = self.rx.next().await;
            })
            .await?;
        Ok(())
    }
}

async fn config() -> String {
    CONFIG.read().unwrap().test.to_string()
}

async fn health() -> &'static str {
    "Healthy"
}
