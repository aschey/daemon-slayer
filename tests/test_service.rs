use assert_cmd::Command;
use daemon_slayer::client::{Manager, ServiceManager, Status};
use std::{thread, time::Duration};

#[test]
fn test_service() {
    let manager = ServiceManager::builder("daemon_slayer_test_service")
        .build()
        .unwrap();
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

    Command::cargo_bin("bin_fixture")
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

    Command::cargo_bin("bin_fixture")
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

    Command::cargo_bin("bin_fixture")
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
