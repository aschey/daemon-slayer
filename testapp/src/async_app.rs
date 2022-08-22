use daemon_slayer::{
    async_trait::async_trait,
    define_service,
    platform::Manager,
    service_config::ServiceConfig,
    service_manager::{ServiceHandler, ServiceManager, StopHandler},
};

define_service!(run_service, Handler);

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


pub async fn run_app() {
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
        },
        _ => {
            #[cfg(feature = "direct")]
            run_service_main().await;
        }
    }
}
