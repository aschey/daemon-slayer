use daemon_slayer::cli::{Action, CliHandlerSync, CliSync};
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{HandlerSync, ServiceSync, StopHandlerSync};
use std::error::Error;
use std::time::{Duration, Instant};
use tracing::info;

pub fn main() {
    let manager = ServiceManager::builder(ServiceHandler::get_service_name())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = CliSync::new(manager, ServiceHandler::new());
    let mut _logger_guard: Option<LoggerGuard> = None;
    if cli.action_type() == Action::Server {
        let (logger, guard) = LoggerBuilder::new(ServiceHandler::get_service_name()).build();
        _logger_guard = Some(guard);
        logger.init();
    }

    cli.handle_input().unwrap();
}

#[derive(daemon_slayer::server::ServiceSync)]
pub struct ServiceHandler {
    tx: std::sync::mpsc::Sender<()>,
    rx: std::sync::mpsc::Receiver<()>,
}

impl HandlerSync for ServiceHandler {
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self { tx, rx }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_test_service_sync"
    }

    fn get_stop_handler(&mut self) -> StopHandlerSync {
        let tx = self.tx.clone();
        Box::new(move || {
            tx.send(()).unwrap();
        })
    }

    fn run_service<F: FnOnce() + Send>(self, on_started: F) -> Result<(), Box<dyn Error>> {
        on_started();
        self.rx.recv().unwrap();
        Ok(())
    }
}
