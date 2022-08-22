use daemon_slayer::{
    define_service,
    platform::Manager,
    service_config::ServiceConfig,
    service_manager::{ServiceHandler, ServiceManager, StopHandler},
};

define_service!(run_service, Handler);

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

pub fn run_app() {
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
        "-r" => run_service(),
        _ => {
            run_service_main();
        }
    }
}
