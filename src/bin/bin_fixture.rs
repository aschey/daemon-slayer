#[cfg(feature = "cli")]
use daemon_slayer::cli::Cli;
#[cfg(feature = "logging")]
use daemon_slayer::logging::LoggerBuilder;
use daemon_slayer::{
    cli::CliCommand,
    platform::Manager,
    service_manager::{Service, ServiceHandler, ServiceManager, StopHandler},
};
#[cfg(feature = "async-tokio")]
use futures::{SinkExt, StreamExt};
use tracing::info;
#[cfg(feature = "logging")]
use tracing_subscriber::util::SubscriberInitExt;

#[maybe_async::sync_impl]
pub fn main() {
    #[cfg(feature = "logging")]
    {
        let (logger, _guard) = LoggerBuilder::new(Handler::get_service_name()).build();
        logger.init();
    }

    let manager = Manager::builder(Handler::get_service_name())
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    #[cfg(feature = "cli")]
    {
        let cli = Cli::<Handler>::new(manager);
        cli.handle_input();
    }
    #[cfg(not(feature = "cli"))]
    {
        let args: Vec<String> = std::env::args().collect();
        let arg = if args.len() > 1 { &args[1] } else { "" };
        match arg {
            "install" => {
                manager.install().unwrap();
                manager.start().unwrap();
            }
            "start" => {
                manager.start().unwrap();
            }
            "stop" => {
                manager.stop().unwrap();
            }
            "status" => {
                println!("{:?}", manager.query_status().unwrap());
            }
            "uninstall" => {
                manager.stop().unwrap();
                manager.uninstall().unwrap();
            }
            "run" => {
                Handler::run_service_main();
            }
            _ => {
                #[cfg(feature = "direct")]
                {
                    let handler = Handler::new();
                    handler.run_service_direct();
                }
            }
        }
    }
}

#[maybe_async::async_impl]
pub fn main() {
    #[cfg(feature = "logging")]
    let (logger, _guard) = LoggerBuilder::new(Handler::get_service_name()).build();
    #[cfg(feature = "logging")]
    logger.init();

    info!("running main");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        //.with_service_level(ServiceLevel::User);
        let manager = Manager::builder(Handler::get_service_name())
            .with_description("test service")
            .with_args(["run"])
            .build()
            .unwrap();
        #[cfg(feature = "cli")]
        {
            let cli = Cli::<Handler>::new(manager);
            cli.handle_input().await.unwrap();
        }
        #[cfg(not(feature = "cli"))]
        {
            let args: Vec<String> = std::env::args().collect();
            let arg = if args.len() > 1 { &args[1] } else { "" };
            match arg {
                "install" => {
                    manager.install().unwrap();
                    manager.start().unwrap();
                }
                "start" => {
                    println!("here");
                    manager.start().unwrap();
                }
                "stop" => {
                    manager.stop().unwrap();
                }
                "status" => {
                    println!("{:?}", manager.query_status().unwrap());
                }
                "uninstall" => {
                    manager.stop().unwrap();
                    manager.uninstall().unwrap();
                }
                "run" => {
                    Handler::run_service_main().await;
                }
                _ => {
                    #[cfg(feature = "direct")]
                    {
                        let handler = Handler::new();
                        handler.run_service_direct().await;
                    }
                }
            }
        }
    });
}

#[maybe_async::sync_impl]
#[derive(daemon_slayer_macros::Service)]
pub struct Handler {
    tx: std::sync::mpsc::Sender<()>,
    rx: std::sync::mpsc::Receiver<()>,
}

#[maybe_async::async_impl]
#[derive(daemon_slayer_macros::Service)]
pub struct Handler {
    tx: futures::channel::mpsc::Sender<()>,
    rx: futures::channel::mpsc::Receiver<()>,
}

#[maybe_async::maybe_async]
impl ServiceHandler for Handler {
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
