use assert_cmd::Command;
use daemon_slayer::client::State;
use daemon_slayer::client::{self, config::Level};
use std::{thread, time::Duration};

#[test]
fn run() {
    if cfg!(windows) || std::env::var("RUN_AS_SYSTEM") == Ok("true".to_owned()) {
        run_tests(false);
    } else {
        run_tests(true);
    }
}

fn run_tests(is_user_service: bool) {
    let bin_name = "test_app";
    let port = 3002;
    let manager = client::config::Builder::new(
        "com.test.daemon_slayer_test".parse().unwrap(),
        assert_cmd::cargo::cargo_bin(bin_name).try_into().unwrap(),
    )
    .with_service_level(if is_user_service {
        Level::User
    } else {
        Level::System
    })
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

    run_manager_cmd(bin_name, "install", is_user_service, || {
        let info = manager.info().unwrap();
        println!("Waiting for install: {info:?}");
        info.state == State::Stopped && info.autostart == Some(false) && info.pid.is_none()
    });

    run_manager_cmd(bin_name, "start", is_user_service, || {
        let info = manager.info().unwrap();
        println!("Waiting for start: {info:?}");
        info.state == State::Started && info.autostart == Some(false) && info.pid.is_some()
    });

    run_manager_cmd(bin_name, "enable", is_user_service, || {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart: {autostart:?}");
        autostart
    });

    wait_for(|| {
        let response = reqwest::blocking::get(format!("http://127.0.0.1:{port}/test"));
        println!("Waiting for service response: {response:?}");
        if let Ok(response) = response {
            return response.text().unwrap() == "test";
        }
        false
    });

    run_manager_cmd(bin_name, "disable", is_user_service, || {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart disable: {autostart:?}");
        !autostart
    });

    run_manager_cmd(bin_name, "stop", is_user_service, || {
        let info = manager.info().unwrap();
        println!("Waiting for stop: {info:?}");
        info.state == State::Stopped && info.autostart == Some(false) && info.pid.is_none()
    });

    run_manager_cmd(bin_name, "uninstall", is_user_service, || {
        let info = manager.info().unwrap();
        println!("Waiting for uninstall: {info:?}");
        info.state == State::NotInstalled && info.autostart.is_none() && info.pid.is_none()
    });

    wait();
}

fn wait() {
    thread::sleep(Duration::from_millis(100));
}

fn run_manager_cmd(bin_name: &str, cmd: &str, is_user_service: bool, condition: impl Fn() -> bool) {
    Command::cargo_bin(bin_name)
        .unwrap()
        .arg(cmd)
        .env("USER_SERVICE", is_user_service.to_string())
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
