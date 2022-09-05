use std::env::args;
use std::time::{Duration, Instant};

use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{Action, CliAsync, CliHandlerAsync, Command};
use daemon_slayer::server::{HandlerAsync, ServiceAsync, StopHandlerAsync};

use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use futures::{SinkExt, StreamExt};
use tracing::info;

use tracing_subscriber::util::SubscriberInitExt;

pub fn main() {
    let logger_builder = LoggerBuilder::new(ServiceHandler::get_service_name());
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        //.with_service_level(ServiceLevel::User);
        let manager = ServiceManager::builder(ServiceHandler::get_service_name())
            .with_description("test service")
            .with_args(["run"])
            .build()
            .unwrap();

        let cli = CliAsync::<ServiceHandler>::new(manager);

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
        "daemon_slayer_test_service"
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

    async fn run_service<F: FnOnce() + Send>(mut self, on_started: F) -> u32 {
        info!("running service");
        on_started();
        loop {
            match tokio::time::timeout(Duration::from_secs(1), self.rx.next()).await {
                Ok(_) => {
                    info!("stopping service");
                    return 0;
                }
                Err(_) => {
                    info!("Current time: {:?}", Instant::now());
                }
            }
        }
    }
}
