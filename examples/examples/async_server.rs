use std::env::args;
use std::error::Error;
use std::time::{Duration, Instant};

use daemon_slayer::cli::{Action, ActionType, CliAsync, Command};
use daemon_slayer::server::{EventHandlerAsync, HandlerAsync, ServiceAsync};

use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use futures::{SinkExt, StreamExt};
use tracing::info;

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let logger_builder = LoggerBuilder::new(ServiceHandler::get_service_name());
    run_async(logger_builder)
}

#[tokio::main]
pub async fn run_async(logger_builder: LoggerBuilder) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = CliAsync::for_server(
        ServiceHandler::new(),
        "daemon_slayer_test_service".to_owned(),
        "test_service".to_owned(),
    );

    let mut _logger_guard: Option<LoggerGuard> = None;

    if cli.action().action_type == ActionType::Server {
        let (logger, guard) = logger_builder.with_ipc_logger(true).build().unwrap();
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
        "daemon_slayer_async_server"
    }

    fn get_event_handler(&mut self) -> EventHandlerAsync {
        let tx = self.tx.clone();
        Box::new(move |event| {
            info!("Received event {event:?}");
            let mut tx = tx.clone();
            Box::pin(async move {
                info!("stopping");
                tx.send(()).await?;
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
