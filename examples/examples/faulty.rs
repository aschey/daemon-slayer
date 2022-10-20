// use std::env::args;
// use std::error::Error;
// use std::time::{Duration, Instant};

// use daemon_slayer::client::{Level, Manager, ServiceManager};

// use daemon_slayer::cli::{Action, ActionType, CliAsync, Command};
// use daemon_slayer::error_handler::ErrorHandler;
// use daemon_slayer::server::{EventHandlerAsync, HandlerAsync, ServiceAsync};

// use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};

// use daemon_slayer::client::health_check::IpcHealthCheckAsync;
// use daemon_slayer::server::IpcHealthCheckServer;
// use futures::{SinkExt, StreamExt};
// use tracing::info;

// use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;

// pub fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
//     daemon_slayer::logging::init_local_time();
//     run_async()
// }

// TODO: re-enable after adding functionality to health check client
fn main() {}

// #[tokio::main]
// pub async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
//     let manager = ServiceManager::builder(ServiceHandler::get_service_name())
//         .with_description("test service")
//         .with_service_level(if cfg!(windows) {
//             Level::System
//         } else {
//             Level::User
//         })
//         .with_autostart(true)
//         .with_args(["run"])
//         .build()
//         .unwrap();

//     let health_check_server = IpcHealthCheckServer::new(ServiceHandler::get_service_name());
//     let health_check_client = IpcHealthCheckAsync::new(health_check_server.sock_path());
//     let cli = CliAsync::builder_for_all(manager, ServiceHandler::new())
//         .with_health_check(Box::new(health_check_client))
//         .build();

//     if cli.action().action_type == ActionType::Server {
//         tokio::spawn(async move {
//             // Simulate a health check that fails sporadically
//             loop {
//                 let handle = health_check_server.spawn_server();
//                 tokio::time::sleep(Duration::from_secs(2)).await;
//                 handle.abort();
//                 tokio::time::sleep(Duration::from_secs(2)).await;
//             }
//         });
//     }

//     let (logger, _guard) = cli
//         .configure_logger()
//         .with_ipc_logger(true)
//         .build()
//         .unwrap();
//     logger.init();

//     cli.configure_error_handler().install()?;

//     cli.handle_input().await?;
//     Ok(())
// }

// #[derive(daemon_slayer::server::ServiceAsync)]
// pub struct ServiceHandler {
//     tx: futures::channel::mpsc::Sender<()>,
//     rx: futures::channel::mpsc::Receiver<()>,
// }

// #[async_trait::async_trait]
// impl HandlerAsync for ServiceHandler {
//     fn new() -> Self {
//         let (tx, rx) = futures::channel::mpsc::channel(32);
//         Self { tx, rx }
//     }

//     fn get_service_name<'a>() -> &'a str {
//         "daemon_slayer_faulty"
//     }

//     fn get_event_handler(&mut self) -> EventHandlerAsync {
//         let tx = self.tx.clone();
//         Box::new(move |event| {
//             let mut tx = tx.clone();
//             Box::pin(async move {
//                 info!("stopping");
//                 let _ = tx.send(()).await;
//                 Ok(())
//             })
//         })
//     }

//     async fn run_service<F: FnOnce() + Send>(
//         mut self,
//         on_started: F,
//     ) -> Result<(), Box<dyn Error + Send + Sync>> {
//         info!("running service");
//         on_started();
//         for _ in 0..5 {
//             match tokio::time::timeout(Duration::from_secs(1), self.rx.next()).await {
//                 Ok(_) => {
//                     info!("stopping service");
//                     return Ok(());
//                 }
//                 Err(_) => {
//                     info!("Current time: {:?}", Instant::now());
//                 }
//             }
//         }
//         panic!("Fatal error!");
//     }
// }
