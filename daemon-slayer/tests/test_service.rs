use std::{env, thread, time::Duration};

use assert_cmd::Command;
use daemon_slayer::windows::Manager;
use windows_service::service::ServiceState;

#[test]
fn test_service() {
    // Run a build to ensure test app is up-to-date
    Command::new("cargo")
        .arg("build")
        .current_dir(env::var("CARGO_WORKSPACE_DIR").unwrap())
        .output()
        .unwrap();

    let manager = Manager::new("daemon_slayer_test_service");
    if manager.is_installed() {
        manager.stop();
        manager.uninstall();

        loop {
            let status = manager.query_status();
            println!("Waiting for uninstall: {status:?}");
            if status.is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    Command::cargo_bin("testapp")
        .unwrap()
        .arg("-i")
        .output()
        .unwrap();

    loop {
        let status = manager.query_status();
        println!("Waiting for start: {status:?}");
        if let Ok(status) = status {
            if status.current_state == ServiceState::Running {
                break;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    manager.stop();

    loop {
        let status = manager.query_status();
        println!("Waiting for stop: {status:?}");
        if let Ok(status) = status {
            if status.current_state == ServiceState::Stopped {
                break;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    Command::cargo_bin("testapp")
        .unwrap()
        .arg("-u")
        .output()
        .unwrap();
    loop {
        let status = manager.query_status();
        println!("Waiting for uninstall: {status:?}");
        if status.is_err() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
}
