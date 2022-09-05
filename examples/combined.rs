use daemon_slayer::cli::{Action, Cli, CliHandler};
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{Handler, Service, StopHandler};
use std::time::{Duration, Instant};
use tracing::info;
use tracing_subscriber::util::SubscriberInitExt;

pub fn main() {
    let manager = ServiceManager::builder(ServiceHandler::get_service_name())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = Cli::<ServiceHandler>::new(manager);
    let mut _logger_guard: Option<LoggerGuard> = None;
    if cli.action_type() == Action::Server {
        let (logger, guard) = LoggerBuilder::new(ServiceHandler::get_service_name()).build();
        _logger_guard = Some(guard);
        logger.init();
    }

    cli.handle_input().unwrap();
}

#[derive(Service)]
pub struct ServiceHandler {
    tx: std::sync::mpsc::Sender<()>,
    rx: std::sync::mpsc::Receiver<()>,
}

impl Handler for ServiceHandler {
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self { tx, rx }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_test_service"
    }

    fn get_stop_handler(&mut self) -> StopHandler {
        let tx = self.tx.clone();
        Box::new(move || {
            tx.send(()).unwrap();
        })
    }

    fn run_service<F: FnOnce() + Send>(self, on_started: F) -> u32 {
        on_started();
        loop {
            match self.rx.recv_timeout(Duration::from_secs(1)) {
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
