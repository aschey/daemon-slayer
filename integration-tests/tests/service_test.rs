use assert_cmd::Command;
use daemon_slayer::client::{Manager, ServiceManager, State};
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
    if manager.info().unwrap().state != State::NotInstalled {
        manager.stop().unwrap();
        thread::sleep(Duration::from_millis(100));
        manager.uninstall().unwrap();
        thread::sleep(Duration::from_millis(100));

        loop {
            let status = manager.info().unwrap().state;
            println!("Waiting for uninstall: {status:?}");
            if status == State::NotInstalled {
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
        let state = manager.info().unwrap().state;
        println!("Waiting for install: {state:?}");
        if state != State::NotInstalled {
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
        let state = manager.info().unwrap().state;
        println!("Waiting for start: {state:?}");
        if state == State::Started {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    Command::cargo_bin(bin_name)
        .unwrap()
        .arg("enable")
        .output()
        .unwrap();

    loop {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart: {autostart:?}");
        if autostart {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    Command::cargo_bin(bin_name)
        .unwrap()
        .arg("disable")
        .output()
        .unwrap();

    loop {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart disable: {autostart:?}");
        if !autostart {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    manager.stop().unwrap();

    loop {
        let state = manager.info().unwrap().state;
        println!("Waiting for stop: {state:?}");
        if state == State::Stopped {
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
        let state = manager.info().unwrap().state;
        println!("Waiting for uninstall: {state:?}");
        if state == State::NotInstalled {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    thread::sleep(Duration::from_millis(100));
}
