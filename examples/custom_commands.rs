use std::env::args;
use std::error::Error;
use std::time::{Duration, Instant};

use daemon_slayer::client::{Manager, ServiceManager};

use daemon_slayer::cli::{clap, Action, CliAsync, CliHandlerAsync, InputState};
use daemon_slayer::server::{HandlerAsync, ServiceAsync, StopHandlerAsync};

use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

use daemon_slayer_client::Level;
use futures::{SinkExt, StreamExt};
use tracing::info;

use tracing_subscriber::util::SubscriberInitExt;

pub fn main() -> Result<(), Box<dyn Error>> {
    let logger_builder = LoggerBuilder::new(ServiceHandler::get_service_name());
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let manager = ServiceManager::builder(ServiceHandler::get_service_name())
            .with_description("test service")
            .with_service_level(if cfg!(windows) {
                Level::System
            } else {
                Level::User
            })
            .with_autostart(true)
            .with_args(["run"])
            .build()
            .unwrap();

        let base_command = clap::Command::default()
            .subcommand(clap::Command::new("custom").about("custom subcommand"))
            .arg(
                clap::Arg::new("custom_arg")
                    .short('c')
                    .long("custom")
                    .help("custom arg"),
            );
        let cli = CliAsync::builder(manager, ServiceHandler::new())
            .with_base_command(base_command)
            .build();

        let mut _logger_guard: Option<LoggerGuard> = None;

        if cli.action_type() == Action::Server {
            let (logger, guard) = logger_builder.with_ipc_logger(true).build();
            _logger_guard = Some(guard);
            logger.init();
        }

        if let InputState::Unhandled(matches) = cli.handle_input().await? {
            if let Some(("custom", _)) = matches.subcommand() {
                println!("matched custom command");
            }
            if let Some(arg) = matches.get_one::<String>("custom_arg") {
                println!("custom arg is {}", arg);
            }
        }
        Ok(())
    })
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
        "daemon_slayer_custom_command"
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
    ) -> Result<(), Box<dyn Error>> {
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
