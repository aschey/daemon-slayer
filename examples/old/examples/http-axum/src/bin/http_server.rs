use anyhow::anyhow;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::Router;
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::error_handler::{cli::ErrorHandlerCliProvider, ErrorHandler};
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::LogTarget;
use daemon_slayer::server::SignalHandler;
use daemon_slayer::signals::SignalListener;
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use daemon_slayer::cli::Cli;
use daemon_slayer::file_watcher::{FileWatcher, FileWatcherBuilder};
use daemon_slayer::logging::{cli::LoggingCliProvider, LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, FutureExt, Handler, Service,
    ServiceContext, SubsystemHandle,
};
use daemon_slayer::task_queue::{
    CancellationToken, Decode, Encode, JobError, JobProcessor, TaskQueue, TaskQueueBuilder,
    TaskQueueClient, Xid,
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
    let logger_builder = LoggerBuilder::new("daemon_slayer_axum")
        .with_default_log_level(tracing::Level::TRACE)
        .with_level_filter(LevelFilter::TRACE)
        .with_target_directive(LogTarget::EventLog, LevelFilter::INFO.into())
        .with_env_filter_directive("sqlx=info".parse()?)
        .with_ipc_logger(true);

    let logging_provider = LoggingCliProvider::new(logger_builder);

    let cli = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::default())
        .with_provider(logging_provider.clone())
        .with_provider(ErrorHandlerCliProvider::default())
        .initialize();

    let (logger, _guard) = logging_provider.get_logger();
    logger.init();

    cli.handle_input().await;

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    subsys: SubsystemHandle,
    task_queue_client: TaskQueueClient,
}

#[async_trait::async_trait]
impl Handler for ServiceHandler {
    async fn new(mut context: ServiceContext) -> Self {
        let subsys = context.get_subsystem_handle();
        context
            .add_event_service::<SignalListener>(SignalListener::all())
            .await;
        let task_queue_client = context
            .spawn(
                TaskQueue::builder()
                    .with_job_handler(MyJob {})
                    .build()
                    .await,
            )
            .await;
        // let (file_watcher_client, file_watcher_events) = context
        //     .add_event_service::<FileWatcher>(
        //         FileWatcherBuilder::default().with_watch_path(PathBuf::from("../Cargo.toml")),
        //     )
        //     .await;

        Self {
            subsys,
            task_queue_client,
        }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_axum"
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");

        self.subsys.start::<Box<dyn Error + Send + Sync>, _, _>(
            "http_server",
            |subsys| async move {
                info!("running service");

                let app = Router::with_state(self.task_queue_client)
                    .route("/hello/:name", get(greeter))
                    .route("/task", get(start_task))
                    .route("/health", get(health))
                    .layer(TraceLayer::new_for_http());
                let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

                axum::Server::bind(&addr)
                    .serve(app.into_make_service())
                    .with_graceful_shutdown(async {
                        subsys.on_shutdown_requested().await;
                        info!("Got shutdown request");
                    })
                    .await?;
                info!("Server terminated");
                Ok(())
            },
        );
        on_started();
        info!("started");

        self.subsys.on_shutdown_requested().await;
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

struct MyJob {}

#[async_trait::async_trait]
impl JobProcessor for MyJob {
    type Payload = ();
    type Error = anyhow::Error;

    async fn handle(
        &self,
        jid: Xid,
        payload: Self::Payload,
        cancellation_token: CancellationToken,
    ) -> Result<(), Self::Error> {
        for _ in 0..10 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    info!("Did a thing");
                }
                // _ = cancellation_token.cancelled() => {
                //     warn!("job cancelled");
                //     return Ok(());
                // }
            }
        }
        info!("Job completed");
        Ok(())
    }

    fn name() -> &'static str {
        "my_job"
    }
}
