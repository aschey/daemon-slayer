use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use axum::extract::Path;
use axum::routing::get;
use axum::Router;
use daemon_slayer::cli::Cli;
use daemon_slayer::client;
use daemon_slayer::client::cli::ClientCliProvider;

use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::error_handler::cli::ErrorHandlerCliProvider;
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::file_watcher::{FileWatcher, FileWatcherBuilder};
use daemon_slayer::logging::cli::LoggingCliProvider;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use daemon_slayer::client::config::Level;
use daemon_slayer::server::cli::ServerCliProvider;
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, ServiceContext, Signal, SignalHandler,
};
use daemon_slayer::signals::SignalListener;
use futures::{SinkExt, StreamExt};
use serde_derive::Deserialize;
use tracing::info;

use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use tracing::log::Log;

#[tokio::main]
pub async fn main() {
    let mut manager_builder = client::builder(ServiceHandler::label())
        .with_description("test service")
        .with_args(["run"]);

    if let Ok(config_file) = std::env::var("CONFIG_FILE") {
        manager_builder = manager_builder.with_environment_variable("CONFIG_FILE", config_file);
    }

    let manager = manager_builder.build().unwrap();
    let logger_builder = LoggerBuilder::new(ServiceHandler::label());
    let logging_provider = LoggingCliProvider::new(logger_builder);

    let cli = Cli::builder()
        .with_provider(ClientCliProvider::new(manager))
        .with_provider(ServerCliProvider::<ServiceHandler>::default())
        .with_provider(ErrorHandlerCliProvider::default())
        .with_provider(logging_provider.clone())
        .initialize()
        .unwrap();

    let (logger, _guard) = logging_provider.get_logger();
    logger.init();

    cli.handle_input().await.unwrap();
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

static CONFIG: RwLock<Config> = RwLock::new(Config { test: false });

#[derive(Deserialize, Default)]
struct Config {
    test: bool,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    type InputData = ();
    type Error = BoxedError;

    async fn new(mut context: ServiceContext, _input_data: Option<Self::InputData>) -> Self {
        let signal_listener = SignalListener::all();
        let signal_store = signal_listener.get_event_store();
        context.add_service(signal_listener).await;

        if let Ok(config_file) = std::env::var("CONFIG_FILE") {
            let abs_path = PathBuf::from(config_file);
            let file_watcher = FileWatcherBuilder::default()
                .with_watch_path(abs_path)
                .build();
            let file_watcher_events = file_watcher.get_event_store();
            context.add_service(file_watcher).await;
            let mut event_store = file_watcher_events.subscribe_events();
            tokio::spawn(async move {
                while let Some(Ok(files)) = event_store.next().await {
                    info!("reloading");
                    if let Some(file) = files.get(0) {
                        let contents = std::fs::read_to_string(file).unwrap();
                        (*CONFIG.write().unwrap()) = toml::from_str::<Config>(&contents).unwrap();
                    }
                }
            });
        }

        Self { signal_store }
    }

    fn label() -> Label {
        "com.daemonslayer.daemonslayertest".parse().unwrap()
    }

    // fn get_watch_paths(&self) -> Vec<PathBuf> {
    //     match std::env::var("CONFIG_FILE") {
    //         Ok(config_file) => {
    //             let abs_path = PathBuf::from(config_file);
    //             vec![abs_path]
    //         }
    //         Err(_) => {
    //             vec![]
    //         }
    //     }
    // }

    // fn get_event_handler(&mut self) -> EventHandlerAsync {
    //     let tx = self.tx.clone();
    //     Box::new(move |event| {
    //         let mut tx = tx.clone();
    //         Box::pin(async move {
    //             match event {
    //                 Event::SignalReceived(_) => {
    //                     info!("stopping");
    //                     tx.send(()).await?;
    //                 }
    //                 Event::FileChanged(files) => {
    //                     info!("reloading");
    //                     if let Some(file) = files.get(0) {
    //                         let contents = std::fs::read_to_string(file).unwrap();
    //                         (*CONFIG.write().unwrap()) =
    //                             toml::from_str::<Config>(&contents).unwrap();
    //                     }
    //                 }
    //                 _ => {}
    //             }

    //             Ok(())
    //         })
    //     })
    // }

    async fn run_service<F: FnOnce() + Send>(mut self, on_started: F) -> Result<(), Self::Error> {
        info!("running service");

        let app = Router::new()
            .route("/config", get(config))
            .route("/health", get(health));
        let addr = SocketAddr::from(([127, 0, 0, 1], 3002));

        on_started();
        info!("started");
        let mut signal_rx = self.signal_store.subscribe_events();
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                let _ = signal_rx.next().await;
            })
            .await
            .unwrap();
        Ok(())
    }
}

async fn config() -> String {
    CONFIG.read().unwrap().test.to_string()
}

async fn health() -> &'static str {
    "Healthy"
}
