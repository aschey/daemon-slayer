use daemon_slayer::{define_service, windows::Manager};

const SERVICE_NAME: &str = "daemon_slayer_test_service";

define_service!(
    SERVICE_NAME,
    run_service,
    std::sync::mpsc::channel(),
    |sender: &std::sync::mpsc::Sender<()>| sender.send(()).unwrap(),
    handle_service
);

pub fn handle_service(
    _: Vec<std::ffi::OsString>,
    _: std::sync::mpsc::Sender<()>,
    rx: std::sync::mpsc::Receiver<()>,
) -> u32 {
    rx.recv().unwrap();
    0
}

pub fn main() {
    let manager = Manager::new(SERVICE_NAME);
    let args: Vec<String> = std::env::args().collect();
    let arg = if args.len() > 1 { &args[1] } else { "" };
    match arg {
        "-i" => {
            manager.install(vec!["-r"]);
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
        "-r" => run_service(),
        _ => {}
    }
}
