use daemon_slayer::cli::Action;
use daemon_slayer::cli::ActionType;
use daemon_slayer::cli::CliSync;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{EventHandlerSync, HandlerSync, ServiceSync};
use std::error::Error;
use std::time::{Duration, Instant};
use tracing::info;

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = CliSync::for_server(
        ServiceHandler::new(),
        "daemon_slayer_test_service".to_owned(),
        "test service".to_owned(),
    );
    let mut _logger_guard: Option<LoggerGuard> = None;
    if cli.action().action_type == ActionType::Server {
        let (logger, guard) = LoggerBuilder::new(ServiceHandler::get_service_name())
            .build()
            .unwrap();
        _logger_guard = Some(guard);
        logger.init();
    }

    cli.handle_input()?;
    Ok(())
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

    fn get_event_handler(&mut self) -> EventHandlerSync {
        let tx = self.tx.clone();
        Box::new(move |event| {
            let _ = tx.send(());
            Ok(())
        })
    }

    fn run_service<F: FnOnce() + Send>(
        self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        on_started();
        loop {
            match self.rx.recv_timeout(Duration::from_secs(1)) {
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
