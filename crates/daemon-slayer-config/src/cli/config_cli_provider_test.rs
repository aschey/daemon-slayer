use confique::Config;
use daemon_slayer_cli::Cli;
use daemon_slayer_core::config::ConfigWatcher;
use tempfile::tempdir;
use tokio::sync::mpsc;

use crate::{AppConfig, ConfigDir};

use super::ConfigCliProvider;

#[tokio::test]
async fn test_config_path() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();
    let mut buf = Vec::new();

    Cli::builder()
        .with_provider(ConfigCliProvider::new(test_config))
        .initialize_from(["provider", "config", "path"])
        .unwrap()
        .handle_input_with_writer(&mut buf)
        .await
        .unwrap();
    assert_eq!(
        config_dir.join("config.toml").to_str().unwrap(),
        String::from_utf8(buf).unwrap().trim()
    );
}

#[tokio::test]
async fn test_config_validate() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();
    let mut buf = Vec::new();

    Cli::builder()
        .with_provider(ConfigCliProvider::new(test_config))
        .initialize_from(["provider", "config", "validate"])
        .unwrap()
        .handle_input_with_writer(&mut buf)
        .await
        .unwrap();
    assert_eq!("Valid\n", String::from_utf8(buf).unwrap());
}

#[tokio::test]
async fn test_print() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();
    let mut buf = Vec::new();

    Cli::builder()
        .with_provider(ConfigCliProvider::new(test_config))
        .initialize_from(if cfg!(feature = "pretty-print") {
            vec!["provider", "config", "-p"]
        } else {
            vec!["provider", "config"]
        })
        .unwrap()
        .handle_input_with_writer(&mut buf)
        .await
        .unwrap();
    assert_eq!(
        "# Default value: true\n#test = true",
        String::from_utf8(buf).unwrap().trim()
    );
}

#[tokio::test]
async fn test_config_watcher() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();
    std::env::set_var("EDITOR", "true");
    let (tx, mut rx) = mpsc::channel(32);
    let watcher = TestConfigWatcher { tx };
    let cli = Cli::builder()
        .with_provider(ConfigCliProvider::new(test_config).with_config_watcher(watcher))
        .initialize_from(["provider", "config", "edit"])
        .unwrap();
    cli.handle_input().await.unwrap();
    rx.recv().await.unwrap();
}

#[derive(Default, Clone, Config, Debug)]
struct TestConfig {
    #[config(default = true)]
    #[allow(unused)]
    test: bool,
}

#[derive(Clone)]
struct TestConfigWatcher {
    tx: mpsc::Sender<()>,
}

impl ConfigWatcher for TestConfigWatcher {
    fn on_config_changed(&mut self) -> Result<(), std::io::Error> {
        self.tx.try_send(()).unwrap();
        Ok(())
    }
}
