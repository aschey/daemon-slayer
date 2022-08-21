use std::{env, thread, time::Duration};

use assert_cmd::Command;
use daemon_slayer::{
    platform::Manager, service_manager::ServiceManager, service_state::ServiceState,
};

#[test]
fn test_service() {
    // Run a build to ensure test app is up-to-date
    Command::new("cargo")
        .arg("build")
        .current_dir(env::var("CARGO_WORKSPACE_DIR").unwrap())
        .output()
        .unwrap();

    let manager = Manager::new("daemon_slayer_test_service");
    if manager.query_status() != ServiceState::NotInstalled {
        manager.stop();
        manager.uninstall();

        loop {
            let status = manager.query_status();
            println!("Waiting for uninstall: {status:?}");
            if status == ServiceState::NotInstalled {
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
        if status == ServiceState::Started {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    manager.stop();

    loop {
        let status = manager.query_status();
        println!("Waiting for stop: {status:?}");
        if status == ServiceState::Stopped {
            break;
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
        if status == ServiceState::NotInstalled {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
}
