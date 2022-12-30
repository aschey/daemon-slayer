use confique::Config;
use tempfile::tempdir;

use crate::{AppConfig, ConfigFileType};

#[test]
fn test_ensure_created() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config: AppConfig<TestConfig> =
        AppConfig::from_custom_path(ConfigFileType::Toml, &config_dir);
    test_config.ensure_created().unwrap();
    assert!(config_dir.join("config.toml").exists());
}

#[test]
fn test_load_config() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config: AppConfig<TestConfig> =
        AppConfig::from_custom_path(ConfigFileType::Toml, config_dir);
    test_config.ensure_created().unwrap();
    assert!(test_config.read_config().unwrap().load_full().test);
}

#[derive(Default, Clone, Config, Debug)]
struct TestConfig {
    #[config(default = true)]
    test: bool,
}
