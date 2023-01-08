use daemon_slayer::client::State;
use daemon_slayer::client::{self, config::Level};
use daemon_slayer::config::server::ConfigService;
use daemon_slayer::config::{AppConfig, ConfigDir};
use daemon_slayer::core::server::{BackgroundServiceManager, CancellationToken};
use daemon_slayer::server::EventStore;
use futures::{Future, StreamExt};
use integration_tests::TestConfig;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn run() {
    if std::env::var("RUN_AS_SYSTEM") == Ok("true".to_owned()) {
        run_tests(false).await;
    } else if !cfg!(windows) {
        run_tests(true).await;
    }
}

async fn run_tests(is_user_service: bool) {
    let bin_name = "test_app";
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let llvm_cov_target = metadata.target_directory.join("llvm-cov-target");
    let app_config =
        AppConfig::<TestConfig>::builder(ConfigDir::ProjectDir(integration_tests::label()))
            .build()
            .unwrap();

    let config_service = ConfigService::new(app_config.clone());
    let mut config_events = config_service.get_event_store().subscribe_events();

    let mut manager = client::config::Builder::new(
        integration_tests::label(),
        if cfg!(coverage) {
            llvm_cov_target
                .join(if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                })
                .join(bin_name)
                .to_string()
                .try_into()
                .unwrap()
        } else {
            assert_cmd::cargo::cargo_bin(bin_name).try_into().unwrap()
        },
    )
    .with_arg(&integration_tests::service_arg())
    .with_service_level(if is_user_service {
        Level::User
    } else {
        Level::System
    })
    .with_user_config(app_config.clone())
    .with_environment_variable(
        "LLVM_PROFILE_FILE",
        llvm_cov_target
            .join("daemon-slayer-%p-%m.profraw")
            .to_string(),
    )
    .build()
    .unwrap();

    if manager.info().unwrap().state != State::NotInstalled {
        wait_for_async(|| async {
            manager.stop().unwrap();
            wait().await;
            manager.uninstall().unwrap();
            wait().await;
            let state = manager.info().unwrap().state;
            println!("Waiting for uninstall: {state:?}");
            state == State::NotInstalled
        })
        .await;
    }

    let uninstalled_info = manager.info().unwrap();
    assert_eq!(uninstalled_info.state, State::NotInstalled);
    assert_eq!(uninstalled_info.autostart, None);
    assert_eq!(uninstalled_info.pid, None);
    assert_eq!(uninstalled_info.last_exit_code, None);

    app_config.overwrite_config_file().unwrap();

    // Don't start file watcher until after we reset the config
    let background_services = BackgroundServiceManager::new(CancellationToken::new());
    background_services
        .get_context()
        .add_service(config_service)
        .await
        .unwrap();

    manager.install().unwrap();
    wait_for(|| {
        let info = manager.info().unwrap();
        println!("Waiting for install: {info:?}");
        info.state == State::Stopped && info.autostart == Some(false) && info.pid.is_none()
    })
    .await;

    manager.start().unwrap();
    wait_for(|| {
        let info = manager.info().unwrap();
        println!("Waiting for start: {info:?}");
        info.state == State::Started && info.autostart == Some(false) && info.pid.is_some()
    })
    .await;

    manager.enable_autostart().unwrap();
    wait_for(|| {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart: {autostart:?}");
        autostart
    })
    .await;

    wait_for_async(|| async move {
        let response = reqwest::get(integration_tests::address_string() + "/test").await;
        if let Ok(response) = response {
            println!("Waiting for service response: {response:?}");
            return response.text().await.unwrap() == "test";
        }
        false
    })
    .await;

    manager.disable_autostart().unwrap();
    wait_for(|| {
        let autostart = manager.info().unwrap().autostart.unwrap();
        println!("Waiting for autostart disable: {autostart:?}");
        !autostart
    })
    .await;

    assert!(app_config.full_path().exists());
    std::fs::copy("./assets/config.toml", app_config.full_path()).unwrap();
    config_events.next().await.unwrap().unwrap();
    manager.reload_config().unwrap();
    wait_for_async(|| async {
        let response = reqwest::get(integration_tests::address_string() + "/env").await;
        if let Ok(response) = response {
            let text = response.text().await.unwrap();
            println!("Waiting for reload: {text}");
            return text == "test_env";
        }
        false
    })
    .await;

    manager.stop().unwrap();
    wait_for(|| {
        let info = manager.info().unwrap();
        println!("Waiting for stop: {info:?}");
        info.state == State::Stopped && info.autostart == Some(false) && info.pid.is_none()
    })
    .await;

    manager.restart().unwrap();
    wait_for(|| {
        let info = manager.info().unwrap();
        println!("Waiting for restart: {info:?}");
        info.state == State::Started
    })
    .await;

    manager.stop().unwrap();
    wait_for(|| {
        let info = manager.info().unwrap();
        println!("Waiting for stop: {info:?}");
        info.state == State::Stopped && info.autostart == Some(false) && info.pid.is_none()
    })
    .await;

    manager.uninstall().unwrap();
    wait_for(|| {
        let info = manager.info().unwrap();
        println!("Waiting for uninstall: {info:?}");
        info.state == State::NotInstalled && info.autostart.is_none() && info.pid.is_none()
    })
    .await;
    std::fs::remove_file(app_config.full_path()).unwrap();

    wait().await;
}

async fn wait() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}

async fn wait_for(condition: impl Fn() -> bool) {
    for _ in 0..10 {
        if condition() {
            return;
        }
        wait().await;
    }
    panic!("Timed out waiting for the condition")
}

async fn wait_for_async<F: Future<Output = bool>>(condition: impl Fn() -> F) {
    for _ in 0..10 {
        if condition().await {
            return;
        }
        wait().await;
    }
    panic!("Timed out waiting for the condition")
}
