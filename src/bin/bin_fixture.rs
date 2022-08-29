use app::Handler;
#[cfg(feature = "logging")]
use daemon_slayer::logging::LoggerBuilder;
use daemon_slayer::{
    define_service,
    platform::Manager,
    service_manager::{ServiceHandler, ServiceManager},
};
#[cfg(feature = "logging")]
use tracing_subscriber::util::SubscriberInitExt;
#[cfg(not(feature = "async-tokio"))]
pub fn main() {
    #[cfg(feature = "logging")]
    {
        let (logger, _guard) = LoggerBuilder::new(Handler::get_service_name()).build();
        logger.init();
    }

    let manager = Manager::builder(Handler::get_service_name())
        .with_description("test service")
        .with_args(["-r"])
        .build()
        .unwrap();

    let args: Vec<String> = std::env::args().collect();
    let arg = if args.len() > 1 { &args[1] } else { "" };
    match arg {
        "-i" => {
            manager.install().unwrap();
            manager.start().unwrap();
        }
        "-s" => {
            manager.start().unwrap();
        }
        "-h" => {
            manager.stop().unwrap();
        }
        "-u" => {
            manager.stop().unwrap();
            manager.uninstall().unwrap();
        }
        "-r" => {
            run_service();
        }
        _ => {
            #[cfg(feature = "direct")]
            run_service_main();
        }
    }
}

#[cfg(feature = "async-tokio")]
pub fn main() {
    #[cfg(feature = "logging")]
    {
        let (logger, _guard) = LoggerBuilder::new(Handler::get_service_name()).build();
        logger.init();
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        //.with_service_level(ServiceLevel::User);
        let manager = Manager::builder(Handler::get_service_name())
            .with_description("test service")
            .with_args(["-r"])
            .build()
            .unwrap();

        let args: Vec<String> = std::env::args().collect();
        let arg = if args.len() > 1 { &args[1] } else { "" };
        match arg {
            "-i" => {
                manager.install().unwrap();
            }
            "-s" => {
                manager.start().unwrap();
            }
            "-h" => {
                manager.stop().unwrap();
            }
            "-u" => {
                manager.stop().unwrap();
                manager.uninstall().unwrap();
            }
            "-r" => {
                run_service().await;
            }
            "-q" => {
                println!("{:?}", manager.query_status());
            }
            _ => {
                #[cfg(feature = "direct")]
                run_service_main().await;
            }
        }
    });
}

define_service!(run_service, Handler);

#[cfg(not(feature = "async-tokio"))]
mod app {
    use daemon_slayer::service_manager::{ServiceHandler, StopHandler};
    pub struct Handler {
        tx: std::sync::mpsc::Sender<()>,
        rx: std::sync::mpsc::Receiver<()>,
    }

    impl ServiceHandler for Handler {
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
            self.rx.recv().unwrap();
            0
        }
    }
}

#[cfg(feature = "async-tokio")]
mod app {
    use async_trait::async_trait;
    use daemon_slayer::service_manager::{ServiceHandler, StopHandler};
    use futures::{SinkExt, StreamExt};
    use tracing::info;
    pub struct Handler {
        tx: futures::channel::mpsc::Sender<()>,
        rx: futures::channel::mpsc::Receiver<()>,
    }
    #[async_trait]
    impl ServiceHandler for Handler {
        fn new() -> Self {
            let (tx, rx) = futures::channel::mpsc::channel(32);
            Self { tx, rx }
        }
        fn get_service_name<'a>() -> &'a str {
            "daemon_slayer_test_service"
        }

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

        async fn run_service<F: FnOnce() + Send>(mut self, on_started: F) -> u32 {
            info!("running service");
            on_started();
            self.rx.next().await;
            0
        }
    }
}
