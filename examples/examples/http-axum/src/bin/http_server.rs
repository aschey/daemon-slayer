use anyhow::anyhow;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::Router;
use daemon_slayer::client::health_check::{HttpHealthCheckAsync, HttpRequestType};
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::signals::{Signal, SignalBuilder, SignalHandler};
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use daemon_slayer::cli::{Action, BuilderAsync, CliAsync, Command};
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, Receiver, ServiceAsync, ServiceContext,
};
use daemon_slayer::task_queue::{
    Decode, Encode, JobError, JobProcessor, TaskQueue, TaskQueueBuilder, TaskQueueClient, Xid,
};
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
    let cli = CliAsync::builder_for_server(
        "daemon_slayer_axum".to_owned(),
        "daemon slayer axum".to_owned(),
        "test service".to_owned(),
    )
    .with_health_check(Box::new(HttpHealthCheckAsync::new(
        HttpRequestType::Get,
        "http://127.0.0.1:3000/health",
    )?))
    .build();

    let (logger, _guard) = cli
        .configure_logger()
        .with_default_log_level(tracing::Level::TRACE)
        .with_level_filter(LevelFilter::TRACE)
        .with_env_filter_directive("sqlx=info".parse()?)
        .with_ipc_logger(true)
        .build()?;
    logger.init();

    cli.configure_error_handler().install()?;

    ServiceHandler::run_service_direct().await?;
    Ok(())
}

#[derive(daemon_slayer::server::ServiceAsync)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
    task_queue_client: TaskQueueClient,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    async fn new(context: &mut ServiceContext) -> Self {
        let (_, signal_store) = context
            .add_event_service::<SignalHandler>(SignalBuilder::all())
            .await;
        let task_queue_client = context
            .add_service::<TaskQueue>(TaskQueueBuilder::default().with_job_handler(MyJob {
                signal_store: signal_store.clone(),
            }))
            .await;
        Self {
            signal_store,
            task_queue_client,
        }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_axum"
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        // context: ServiceContextAsync,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");

        let signal_store = self.signal_store.clone();
        let app = Router::with_state(self.task_queue_client)
            .route("/hello/:name", get(greeter))
            .route("/task", get(start_task))
            .route("/health", get(health))
            .layer(TraceLayer::new_for_http());
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        on_started();
        info!("started");
        let mut shutdown_rx = self.signal_store.subscribe_events();
        let mut shutdown_rx_ = self.signal_store.subscribe_events();

        let (finished_tx, mut finished_rx) = tokio::sync::mpsc::channel(32);
        let handle = tokio::spawn(async move {
            axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .with_graceful_shutdown(async {
                    let r = shutdown_rx_.recv().await;
                    info!("Got shutdown {r:?}");
                })
                .await
                .unwrap();
            finished_tx.send(()).await.unwrap();
        });

        tokio::select! {
            _ = handle => {},
            _ = shutdown_rx.recv() => {
                if (tokio::time::timeout(Duration::from_millis(100), finished_rx.recv()).await).is_err() {
                    warn!("Server didn't shut down, forcing termination");
                }
            }
        };

        info!("Server terminated");
        Ok(())
    }
}

async fn greeter(Path(name): Path<String>) -> String {
    format!("Hello {name}")
}

async fn start_task(State(queue): State<TaskQueueClient>) -> String {
    let res = queue.schedule::<MyJob>((), 0).await;
    res.to_string()
}

async fn health() -> &'static str {
    "Healthy"
}

struct MyJob {
    signal_store: BroadcastEventStore<Signal>,
}

#[async_trait::async_trait]
impl JobProcessor for MyJob {
    type Payload = ();
    type Error = anyhow::Error;

    async fn handle(&self, jid: Xid, payload: Self::Payload) -> Result<(), Self::Error> {
        let mut event_rx = self.signal_store.subscribe_events();
        for _ in 0..10 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    info!("Did a thing");
                }
                _ = event_rx.recv() => {
                    warn!("Job cancelled");
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn name() -> &'static str {
        "my_job"
    }
}
