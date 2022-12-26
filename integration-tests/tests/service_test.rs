use assert_cmd::Command;
use daemon_slayer::client::State;
use daemon_slayer::client::{self, config::Level};
use daemon_slayer::config::{AppConfig, ConfigFileType};
use integration_tests::TestConfig;
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

    let manager = client::config::Builder::new(
        integration_tests::label(),
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

    let app_config =
        AppConfig::<TestConfig>::from_config_dir(integration_tests::label(), ConfigFileType::Toml)
            .unwrap();
    if app_config.full_path().exists() {
        std::fs::remove_file(app_config.full_path()).unwrap();
    }

    app_config.ensure_config_file().unwrap();
    assert!(app_config.full_path().exists());

    let uninstalled_info = manager.info().unwrap();
    assert_eq!(uninstalled_info.state, State::NotInstalled);
    assert_eq!(uninstalled_info.autostart, None);
    assert_eq!(uninstalled_info.pid, None);
    assert_eq!(uninstalled_info.last_exit_code, None);

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
        let response = reqwest::blocking::get(integration_tests::address_string() + "/test");
        if let Ok(response) = response {
            println!("Waiting for service response: {response:?}");
            return response.text().unwrap() == "test";
        }
        false
    });

    run_manager_cmd(bin_name, "disable", is_user_service, || {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart disable: {autostart:?}");
        !autostart
    });

    std::fs::copy("./assets/config.toml", app_config.full_path()).unwrap();

    run_manager_cmd(bin_name, "reload", is_user_service, || {
        let response = reqwest::blocking::get(integration_tests::address_string() + "/env");
        if let Ok(response) = response {
            let text = response.text().unwrap();
            println!("Waiting for reload: {text}");
            return text == "test_env";
        }
        false
    });

    run_manager_cmd(bin_name, "stop", is_user_service, || {
        let info = manager.info().unwrap();
        println!("Waiting for stop: {info:?}");
        info.state == State::Stopped && info.autostart == Some(false) && info.pid.is_none()
    });

    run_manager_cmd(bin_name, "restart", is_user_service, || {
        let info = manager.info().unwrap();
        println!("Waiting for restart: {info:?}");
        info.state == State::Started
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
    std::fs::remove_file(app_config.full_path()).unwrap();
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
