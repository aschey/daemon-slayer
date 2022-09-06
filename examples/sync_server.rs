use daemon_slayer::cli::{Action, CliHandlerSync, ServerCliSync};
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{HandlerSync, ServiceSync, StopHandlerSync};
use std::time::{Duration, Instant};
use tracing::info;
use tracing_subscriber::util::SubscriberInitExt;

pub fn main() {
    let cli = ServerCliSync::<ServiceHandler>::new(
        "daemon_slayer_test_service".to_owned(),
        "test service".to_owned(),
    );
    let mut _logger_guard: Option<LoggerGuard> = None;
    if cli.action_type() == Action::Server {
        let (logger, guard) = LoggerBuilder::new(ServiceHandler::get_service_name()).build();
        _logger_guard = Some(guard);
        logger.init();
    }

    cli.handle_input().unwrap();
}

#[derive(ServiceSync)]
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
        "daemon_slayer_sync_server"
    }

    fn get_stop_handler(&mut self) -> StopHandlerSync {
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
