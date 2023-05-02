use std::time::Duration;

use confique::Config;
use daemon_slayer_core::{
    server::{BackgroundServiceManager, EventStore},
    CancellationToken,
};
use futures::StreamExt;
use tempfile::tempdir;

use crate::{AppConfig, ConfigDir};

use super::ConfigService;

#[tokio::test]
async fn test_serivce() {
    let cancellation_token = CancellationToken::new();
    let service_manager = BackgroundServiceManager::new(cancellation_token.clone());
    let config_dir = tempdir().unwrap().into_path();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();
    assert!(test_config.snapshot().test);

    let service = ConfigService::new(test_config.clone());
    let mut events = service.get_event_store().subscribe_events();
    service_manager
        .get_context()
        .add_service(service)
        .await
        .unwrap();
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
