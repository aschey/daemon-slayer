use assert_cmd::Command;
use daemon_slayer::client;
use daemon_slayer::client::State;
use std::{thread, time::Duration};

#[test]
fn test_combined() {
    let bin_name = "test_app";
    let port = 3002;
    let manager = client::config::Builder::new(
        "com.test.daemon_slayer_test".parse().unwrap(),
        assert_cmd::cargo::cargo_bin(bin_name).try_into().unwrap(),
    )
    .build()
    .unwrap();
    if manager.info().unwrap().state != State::NotInstalled {
        wait_for(|| {
            manager.stop().unwrap();
            wait();
            manager.uninstall().unwrap();
            wait();
            let state = manager.info().unwrap().state;
            println!("Waiting for uninstall: {state:?}");
            state == State::NotInstalled
        });
    }

    run_manager_cmd(bin_name, "install", || {
        let info = manager.info().unwrap();
        println!("Waiting for install: {info:?}");
        info.state == State::Stopped && info.autostart == Some(false) && info.pid.is_none()
    });

    run_manager_cmd(bin_name, "start", || {
        let info = manager.info().unwrap();
        println!("Waiting for start: {info:?}");
        info.state == State::Started && info.autostart == Some(false) && info.pid.is_some()
    });

    run_manager_cmd(bin_name, "enable", || {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart: {autostart:?}");
        autostart
    });

    wait_for(|| {
        let response = reqwest::blocking::get(format!("http://127.0.0.1:{port}/test"))
            .unwrap()
            .text()
            .unwrap();
        println!("Waiting for service response: {response}");
        response == "test"
    });

    run_manager_cmd(bin_name, "disable", || {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart disable: {autostart:?}");
        !autostart
    });

    run_manager_cmd(bin_name, "stop", || {
        let info = manager.info().unwrap();
        println!("Waiting for stop: {info:?}");
        info.state == State::Stopped && info.autostart == Some(false) && info.pid.is_none()
    });

    run_manager_cmd(bin_name, "uninstall", || {
        let info = manager.info().unwrap();
        println!("Waiting for uninstall: {info:?}");
        info.state == State::NotInstalled && info.autostart.is_none() && info.pid.is_none()
    });

    wait();
}

fn wait() {
    thread::sleep(Duration::from_millis(100));
}

fn run_manager_cmd(bin_name: &str, cmd: &str, condition: impl Fn() -> bool) {
    Command::cargo_bin(bin_name)
        .unwrap()
        .arg(cmd)
        .output()
        .unwrap();
    wait_for(condition);
}

fn wait_for(condition: impl Fn() -> bool) {
    for _ in 0..5 {
        if condition() {
            return;
        }
        wait();
    }
    panic!("Timed out waiting for the condition")
}
