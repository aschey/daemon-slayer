use std::env::args;
use std::error::Error;
use std::time::{Duration, Instant};

use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{Action, CliAsync, CliHandlerAsync, Command};
use daemon_slayer::server::{HandlerAsync, ServiceAsync, StopHandlerAsync};

use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use daemon_slayer::client::Level;
use futures::{SinkExt, StreamExt};
use tracing::info;

use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;

pub fn main() {
    let logger_builder = LoggerBuilder::new(ServiceHandler::get_service_name());
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let manager = ServiceManager::builder(ServiceHandler::get_service_name())
            .with_description("test service")
            .with_args(["run"])
            .build()
            .unwrap();

        let cli = CliAsync::new(manager, ServiceHandler::new());

        let mut _logger_guard: Option<LoggerGuard> = None;

        if cli.action_type() == Action::Server {
            let (logger, guard) = logger_builder.build();
            _logger_guard = Some(guard);
            logger.init();
        }

        cli.handle_input().await.unwrap();
    });
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
        "daemon_slayer_test_service_async"
    }

    fn get_stop_handler(&mut self) -> StopHandlerAsync {
        let tx = self.tx.clone();
        Box::new(move || {
            let mut tx = tx.clone();
            Box::pin(async move {
                info!("stopping");
                tx.send(()).await.unwrap();
            })
        })
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");
        on_started();
        self.rx.next().await;
        Ok(())
    }
}
