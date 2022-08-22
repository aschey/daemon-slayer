use std::{env, thread, time::Duration};

use assert_cmd::Command;
use daemon_slayer::{
    platform::Manager, service_config::ServiceConfig, service_manager::ServiceManager,
    service_status::ServiceStatus,
};

#[test]
fn test_service() {
    // Run a build to ensure test app is up-to-date
    Command::new("cargo")
        .arg("build")
        .current_dir(env::var("CARGO_WORKSPACE_DIR").unwrap())
        .output()
        .unwrap();

    let manager = Manager::new(ServiceConfig::new("daemon_slayer_test_service")).unwrap();
    if manager.query_status().unwrap() != ServiceStatus::NotInstalled {
        manager.stop().unwrap();
        manager.uninstall().unwrap();

        loop {
            let status = manager.query_status().unwrap();
            println!("Waiting for uninstall: {status:?}");
            if status == ServiceStatus::NotInstalled {
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
        let status = manager.query_status().unwrap();
        println!("Waiting for start: {status:?}");
        if status == ServiceStatus::Started {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    manager.stop().unwrap();

    loop {
        let status = manager.query_status().unwrap();
        println!("Waiting for stop: {status:?}");
        if status == ServiceStatus::Stopped {
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
        let status = manager.query_status().unwrap();
        println!("Waiting for uninstall: {status:?}");
        if status == ServiceStatus::NotInstalled {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
}
