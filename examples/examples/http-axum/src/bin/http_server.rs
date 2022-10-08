use axum::extract::{Path, State};
use axum::routing::get;
use axum::Router;
use daemon_slayer::client::health_check::{HttpHealthCheckAsync, HttpRequestType};
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use daemon_slayer::cli::{Action, BuilderAsync, CliAsync, Command};
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{
    Event, EventHandlerAsync, HandlerAsync, ServiceAsync, ServiceConfig, ServiceContextAsync,
};
use daemon_slayer::task_queue::{Decode, Encode, JobError, JobProcessor, TaskQueue, Xid};
use futures::{SinkExt, StreamExt};
use tower_http::trace::TraceLayer;
use tracing::metadata::LevelFilter;
use tracing::{error, info};

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = CliAsync::builder_for_server(
        ServiceHandler::new(),
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
        .with_default_log_level(tracing::Level::INFO)
        .with_level_filter(LevelFilter::INFO)
        .with_ipc_logger(true)
        .build()?;
    logger.init();

    cli.configure_error_handler().install()?;

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

    fn get_event_handler(&mut self) -> EventHandlerAsync {
        let tx = self.tx.clone();
        Box::new(move |event| {
            let mut tx = tx.clone();
            Box::pin(async move {
                match event {
                    Event::SignalReceived(_) => {
                        info!("stopping");
                        if let Err(e) = tx.send(()).await {
                            error!("Error sending stop: {e:?}");
                        }
                        Ok(())
                    }
                    Event::TaskQueueEvent(e) => {
                        info!("Task queue event: {e:?}");
                        Ok(())
                    }
                    _ => Ok(()),
                }
            })
        })
    }

    fn configure(&self, config: &mut ServiceConfig) {
        config.add_job_handler(MyJob);
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        context: ServiceContextAsync,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");

        async fn greeter(Path(name): Path<String>) -> String {
            format!("Hello {name}")
        }

        let task_queue = context.task_queue.clone();

        let app = Router::with_state(task_queue)
            .route("/hello/:name", get(greeter))
            .route("/task", get(start_task))
            .route("/health", get(health))
            .layer(TraceLayer::new_for_http());
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        on_started();
        info!("started");
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                let r = self.rx.next().await;
                info!("Got shutdown {r:?}");
            })
            .await?;
        Ok(())
    }
}

async fn start_task(State(queue): State<TaskQueue>) {
    queue.schedule::<MyJob>(()).await;
}

async fn health() -> &'static str {
    "Healthy"
}

struct MyJob;

#[async_trait::async_trait]
impl JobProcessor for MyJob {
    type Payload = ();
    type Error = anyhow::Error;

    async fn handle(&self, jid: Xid, payload: Self::Payload) -> Result<(), Self::Error> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        info!("Finished job");
        Ok(())
    }

    fn name() -> &'static str {
        "my_job"
    }
}
