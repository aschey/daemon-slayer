use daemon_slayer::cli::{Action, ActionType, CliSync};
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{EventHandlerSync, HandlerSync, ServiceSync};
use std::error::Error;
use std::time::{Duration, Instant};
use tracing::info;

pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ServiceManager::builder(ServiceHandler::get_service_name())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = CliSync::for_all(manager, ServiceHandler::new());

    let (logger, _guard) = LoggerBuilder::new(ServiceHandler::get_service_name())
        .build()
        .unwrap();

    logger.init();

    cli.configure_error_handler().install()?;
    cli.handle_input()?;
    Ok(())
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
        "daemon_slayer_sync_combined"
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
