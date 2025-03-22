use std::time::Duration;

use confique::Config;
use daemon_slayer_core::CancellationToken;
use daemon_slayer_core::server::EventStore;
use daemon_slayer_core::server::background_service::{self, Manager};
use futures::StreamExt;
use tempfile::tempdir;

use super::ConfigService;
use crate::{AppConfig, ConfigDir};

#[tokio::test]
async fn test_serivce() {
    let cancellation_token = CancellationToken::new();
    let service_manager = Manager::new(
        cancellation_token.clone(),
        background_service::Settings::default(),
    );
    let config_dir = tempdir().unwrap().into_path();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();
    assert!(test_config.snapshot().test);

    let service = ConfigService::new(test_config.clone());
    let mut events = service.get_event_store().subscribe_events();
    service_manager.get_context().spawn(service);
    tokio::time::sleep(Duration::from_millis(50)).await;
    std::fs::write(test_config.full_path(), "test = false").unwrap();
    let (current, new) = events.next().await.unwrap().unwrap();
    assert!(current.test);
    assert!(!new.test);
    assert!(!test_config.snapshot().test);

    service_manager.cancel().await.unwrap();
}

#[derive(Default, Clone, Config, Debug)]
struct TestConfig {
    #[config(default = true)]
    test: bool,
}
