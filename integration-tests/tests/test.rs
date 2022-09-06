use assert_cmd::Command;
use daemon_slayer::client::{Manager, ServiceManager, Status};
use std::{thread, time::Duration};

#[test]
fn test_async_combined() {
    test_combined("daemon_slayer_test_service_async", "async_combined");
}

#[test]
fn test_sync_combined() {
    test_combined("daemon_slayer_test_service_sync", "sync_combined");
}

fn test_combined(service_name: &str, bin_name: &str) {
    let manager = ServiceManager::builder(service_name).build().unwrap();
    if manager.query_status().unwrap() != Status::NotInstalled {
        manager.stop().unwrap();
        thread::sleep(Duration::from_millis(100));
        manager.uninstall().unwrap();
        thread::sleep(Duration::from_millis(100));

        loop {
            let status = manager.query_status().unwrap();
            println!("Waiting for uninstall: {status:?}");
            if status == Status::NotInstalled {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    Command::cargo_bin(bin_name)
        .unwrap()
        .arg("install")
        .output()
        .unwrap();

    loop {
        let status = manager.query_status().unwrap();
        println!("Waiting for install: {status:?}");
        if status != Status::NotInstalled {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    Command::cargo_bin(bin_name)
        .unwrap()
        .arg("start")
        .output()
        .unwrap();

    loop {
        let status = manager.query_status().unwrap();
        println!("Waiting for start: {status:?}");
        if status == Status::Started {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    manager.stop().unwrap();

    loop {
        let status = manager.query_status().unwrap();
        println!("Waiting for stop: {status:?}");
        if status == Status::Stopped {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    Command::cargo_bin(bin_name)
        .unwrap()
        .arg("uninstall")
        .output()
        .unwrap();
    loop {
        let status = manager.query_status().unwrap();
        println!("Waiting for uninstall: {status:?}");
        if status == Status::NotInstalled {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    thread::sleep(Duration::from_millis(100));
}
