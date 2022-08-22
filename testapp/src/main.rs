use daemon_slayer::{
    define_service, platform::Manager, service_config::ServiceConfig,
    service_manager::ServiceManager,
};

const SERVICE_NAME: &str = "daemon_slayer_test_service";

define_service!(
    SERVICE_NAME,
    run_service,
    tokio::sync::mpsc::channel(32),
    |sender: &tokio::sync::mpsc::Sender<()>| sender.blocking_send(()).unwrap(),
    handle_service
);

pub async fn handle_service(
    _: tokio::sync::mpsc::Sender<()>,
    mut rx: tokio::sync::mpsc::Receiver<()>,
) -> u32 {
    rx.recv().await.unwrap();
    0
}

#[tokio::main]
pub async fn main() {
    let config = ServiceConfig::new(SERVICE_NAME)
        .with_description("test service")
        .with_args(["-r"]);
    let manager = Manager::new(config);
    let args: Vec<String> = std::env::args().collect();
    let arg = if args.len() > 1 { &args[1] } else { "" };
    match arg {
        "-i" => {
            manager.install();
            manager.start();
        }
        "-s" => {
            manager.start();
        }
        "-h" => {
            manager.stop();
        }
        "-u" => {
            manager.stop();
            manager.uninstall();
        }
        "-r" => run_service().await,
        _ => {}
    }
}
