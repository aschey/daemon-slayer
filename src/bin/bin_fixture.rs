use app::Handler;
use daemon_slayer::{
    define_service,
    platform::Manager,
    service_config::ServiceConfig,
    service_manager::{ServiceHandler, ServiceManager},
};

#[cfg(not(feature = "async-tokio"))]
pub fn main() {
    let config = ServiceConfig::new(Handler::get_service_name())
        .with_description("test service")
        .with_args(["-r"]);
    let manager = Manager::new(config).unwrap();
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
#[tokio::main]
pub async fn main() {
    let config = ServiceConfig::new(Handler::get_service_name())
        .with_description("test service")
        .with_args(["-r"]);
    let manager = Manager::new(config).unwrap();
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
            run_service().await;
        }
        _ => {
            #[cfg(feature = "direct")]
            run_service_main().await;
        }
    }
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

        fn run_service(&mut self) -> u32 {
            self.rx.recv().unwrap();
            0
        }
    }
}

#[cfg(feature = "async-tokio")]
mod app {
    use async_trait::async_trait;
    use daemon_slayer::service_manager::{ServiceHandler, StopHandler};
    pub struct Handler {
        tx: tokio::sync::mpsc::Sender<()>,
        rx: tokio::sync::mpsc::Receiver<()>,
    }
    #[async_trait]
    impl ServiceHandler for Handler {
        fn new() -> Self {
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            Self { tx, rx }
        }
        fn get_service_name<'a>() -> &'a str {
            "daemon_slayer_test_service"
        }

        fn get_stop_handler(&mut self) -> StopHandler {
            let tx = self.tx.clone();
            Box::new(move || {
                let tx = tx.clone();
                Box::pin(async move { tx.send(()).await.unwrap() })
            })
        }

        async fn run_service(&mut self) -> u32 {
            self.rx.recv().await.unwrap();
            0
        }
    }
}
