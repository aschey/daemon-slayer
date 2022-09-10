use axum::routing::get;
use axum::Router;
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer_client::{HttpHealthCheckAsync, RequestType};
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use daemon_slayer::cli::{Action, BuilderAsync, CliAsync, Command};
use daemon_slayer::server::{HandlerAsync, ServiceAsync, StopHandlerAsync};

use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use futures::{SinkExt, StreamExt};
use tower_http::trace::TraceLayer;
use tracing::info;

use tracing::metadata::LevelFilter;
use tracing_subscriber::util::SubscriberInitExt;

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let logger_builder = LoggerBuilder::new(ServiceHandler::get_service_name())
        .with_default_log_level(tracing::Level::TRACE)
        .with_level_filter(LevelFilter::TRACE)
        .with_ipc_logger(true);
    run_async(logger_builder)
}

#[tokio::main]
pub async fn run_async(logger_builder: LoggerBuilder) -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ServiceManager::builder(ServiceHandler::get_service_name())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = CliAsync::builder(manager, ServiceHandler::new())
        .with_health_check(Box::new(HttpHealthCheckAsync::new(
            RequestType::Get,
            "http://127.0.0.1:3000/health",
        )))
        .build();

    let mut _logger_guard: Option<LoggerGuard> = None;

    if cli.action_type() == Action::Server {
        let (logger, guard) = logger_builder.build();
        _logger_guard = Some(guard);
        logger.init();
    }

    cli.handle_input().await?;
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
        "daemon_slayer_axum"
    }

    fn get_stop_handler(&mut self) -> StopHandlerAsync {
        let tx = self.tx.clone();
        Box::new(move || {
            let mut tx = tx.clone();
            Box::pin(async move {
                info!("stopping");
                let _ = tx.send(()).await;
            })
        })
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");

        let app = Router::new()
            .route("/", get(root))
            .route("/health", get(health))
            .layer(TraceLayer::new_for_http());
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

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

async fn root() -> &'static str {
    "Hello, World!"
}

async fn health() -> &'static str {
    "Healthy"
}
