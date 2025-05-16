use confique::Config;
use daemon_slayer_core::Mergeable;
use daemon_slayer_core::config::Accessor;
use tempfile::tempdir;

use crate::{AppConfig, ConfigDir, ConfigFileType};

#[test]
fn test_initial_load() {
    let config_dir = tempdir().unwrap().keep();

    let config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();
    assert!(config_dir.join("config.toml").exists());
    assert!(config.snapshot().test);
}

#[test]
fn test_load_config() {
    let config_dir = tempdir().unwrap().keep();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir))
        .build()
        .unwrap();

    assert!(test_config.read_config().unwrap().load_full().test);
}

#[test]
fn test_with_config_filename() {
    let config_dir = tempdir().unwrap().keep();
    AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .with_config_filename("test.toml")
        .build()
        .unwrap();

    assert!(config_dir.join("test.toml").exists());
}

#[test]
fn test_contents() {
    let config_dir = tempdir().unwrap().keep();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir))
        .build()
        .unwrap();

    assert_eq!(
        "# Default value: true\n#test = true",
        test_config.contents().unwrap().trim()
    );
}

#[test]
fn test_change_file_type() {
    let config_dir = tempdir().unwrap().keep();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .with_config_file_type(ConfigFileType::Json5)
        .build()
        .unwrap();
    assert_eq!(config_dir.join("config.json5"), test_config.full_path());

    assert_eq!(
        "{\n  // Default value: true\n  //test: true,\n}",
        test_config.contents().unwrap().trim()
    );
}

#[test]
fn test_snapshot() {
    let config_dir = tempdir().unwrap().keep();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();

    assert!(test_config.read_config().unwrap().load().test);
    std::fs::write(config_dir.join("config.toml"), "test = false").unwrap();
    assert!(test_config.snapshot().test);
    assert!(!test_config.read_config().unwrap().load().test);
}

#[test]
fn test_overwrite() {
    let config_dir = tempdir().unwrap().keep();
    let test_config = AppConfig::<TestConfig>::builder(ConfigDir::Custom(config_dir.clone()))
        .build()
        .unwrap();

    std::fs::write(config_dir.join("config.toml"), "test = false").unwrap();
    assert!(!test_config.read_config().unwrap().load().test);
    test_config.overwrite_config_file().unwrap();
    assert!(test_config.read_config().unwrap().load().test);
}

#[test]
fn test_access() {
    let config_dir = tempdir().unwrap().keep();
    let test_config = AppConfig::<TestConfig2>::builder(ConfigDir::Custom(config_dir))
        .build()
        .unwrap();

    let nested = test_config.access();
    assert!(nested.snapshot().test);
}

#[derive(Default, Clone, Config, Debug)]
struct TestConfig {
    #[config(default = true)]
    test: bool,
}

#[derive(Default, Clone, Config, Debug)]
struct TestConfig2 {
    #[config(nested)]
    nested: Nested,
}

impl AsRef<Nested> for TestConfig2 {
    fn as_ref(&self) -> &Nested {
        &self.nested
    }
}

#[derive(Mergeable, Config, Default, Clone, Debug)]
struct Nested {
    #[config(default = true)]
    test: bool,
}
