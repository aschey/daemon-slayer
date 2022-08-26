use assert_cmd::Command;
use daemon_slayer::{
    platform::Manager, service_manager::ServiceManager, service_status::ServiceStatus,
};
use std::{thread, time::Duration};

#[test]
fn test_service() {
    let manager = Manager::builder("daemon_slayer_test_service")
        .build()
        .unwrap();
    if manager.query_status().unwrap() != ServiceStatus::NotInstalled {
        manager.stop().unwrap();
        thread::sleep(Duration::from_millis(100));
        manager.uninstall().unwrap();
        thread::sleep(Duration::from_millis(100));

        loop {
            let status = manager.query_status().unwrap();
            println!("Waiting for uninstall: {status:?}");
            if status == ServiceStatus::NotInstalled {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    Command::cargo_bin("bin_fixture")
        .unwrap()
        .arg("-i")
        .output()
        .unwrap();

    loop {
        let status = manager.query_status().unwrap();
        println!("Waiting for install: {status:?}");
        if status != ServiceStatus::NotInstalled {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    Command::cargo_bin("bin_fixture")
        .unwrap()
        .arg("-s")
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

    Command::cargo_bin("bin_fixture")
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
    thread::sleep(Duration::from_millis(100));
}
