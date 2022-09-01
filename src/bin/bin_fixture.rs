use daemon_slayer_cli::Cli;
use daemon_slayer_client::{Manager, ServiceManager};

use daemon_slayer_server::{Handler, Service, StopHandler};
#[cfg(feature = "logging")]
use daemon_slayer_server::{LoggerBuilder, LoggerGuard};
#[cfg(feature = "async-tokio")]
use futures::{SinkExt, StreamExt};
use tracing::info;
#[cfg(feature = "logging")]
use tracing_subscriber::util::SubscriberInitExt;

#[maybe_async::sync_impl]
pub fn main() {
    #[cfg(feature = "logging")]
    {
        let (logger, _guard) = LoggerBuilder::new(ServiceHandler::get_service_name()).build();
        logger.init();
    }

    let manager = ServiceManager::builder(ServiceHandler::get_service_name())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let cli = Cli::<ServiceHandler>::new(manager);
    cli.handle_input().unwrap();
}

#[maybe_async::async_impl]
pub fn main() {
    #[cfg(feature = "logging")]
    let (logger, _guard) = LoggerBuilder::new(ServiceHandler::get_service_name()).build();
    #[cfg(feature = "logging")]
    logger.init();

    info!("running main");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        //.with_service_level(ServiceLevel::User);
        let manager = ServiceManager::builder(ServiceHandler::get_service_name())
            .with_description("test service")
            .with_args(["run"])
            .build()
            .unwrap();
        let cli = Cli::<ServiceHandler>::new(manager);
        cli.handle_input().await.unwrap();
    });
}

#[maybe_async::sync_impl]
#[derive(daemon_slayer_server::Service)]
pub struct ServiceHandler {
    tx: std::sync::mpsc::Sender<()>,
    rx: std::sync::mpsc::Receiver<()>,
}

#[maybe_async::async_impl]
#[derive(daemon_slayer_server::Service)]
pub struct ServiceHandler {
    tx: futures::channel::mpsc::Sender<()>,
    rx: futures::channel::mpsc::Receiver<()>,
}

#[maybe_async::maybe_async]
impl Handler for ServiceHandler {
    #[maybe_async::sync_impl]
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self { tx, rx }
    }

    #[maybe_async::async_impl]
    fn new() -> Self {
        let (tx, rx) = futures::channel::mpsc::channel(32);
        Self { tx, rx }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_test_service"
    }

    #[maybe_async::sync_impl]
    fn get_stop_handler(&mut self) -> StopHandler {
        let tx = self.tx.clone();
        Box::new(move || {
            tx.send(()).unwrap();
        })
    }

    #[maybe_async::async_impl]
    fn get_stop_handler(&mut self) -> StopHandler {
        let tx = self.tx.clone();
        Box::new(move || {
            let mut tx = tx.clone();
            Box::pin(async move {
                info!("stopping");
                tx.send(()).await.unwrap();
            })
        })
    }

    #[maybe_async::sync_impl]
    fn run_service<F: FnOnce() + Send>(self, on_started: F) -> u32 {
        on_started();
        self.rx.recv().unwrap();
        0
    }

    #[maybe_async::async_impl]
    async fn run_service<F: FnOnce() + Send>(mut self, on_started: F) -> u32 {
        info!("running service");
        on_started();

        self.rx.next().await;
        info!("stopping service");
        0
    }
}
