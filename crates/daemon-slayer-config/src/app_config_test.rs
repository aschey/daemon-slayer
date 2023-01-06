use crate::{AppConfig, ConfigFileType};
use confique::Config;
use daemon_slayer_core::config::Accessor;
use daemon_slayer_core::Mergeable;
use tempfile::tempdir;

#[test]
fn test_ensure_created() {
    let config_dir = tempdir().unwrap().into_path();

    AppConfig::<TestConfig>::from_custom_path(ConfigFileType::Toml, &config_dir).unwrap();
    assert!(config_dir.join("config.toml").exists());
}

#[test]
fn test_load_config() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config =
        AppConfig::<TestConfig>::from_custom_path(ConfigFileType::Toml, config_dir).unwrap();

    assert!(test_config.read_config().unwrap().load_full().test);
}

#[test]
fn test_contents() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config =
        AppConfig::<TestConfig>::from_custom_path(ConfigFileType::Toml, config_dir).unwrap();

    assert_eq!(
        "# Default value: true\n#test = true",
        test_config.contents().unwrap().trim()
    );
}

#[test]
fn test_snapshot() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config =
        AppConfig::<TestConfig>::from_custom_path(ConfigFileType::Toml, &config_dir).unwrap();

    assert!(test_config.read_config().unwrap().load().test);
    std::fs::write(config_dir.join("config.toml"), "test = false").unwrap();
    assert!(test_config.snapshot().test);
    assert!(!test_config.read_config().unwrap().load().test);
}

#[test]
fn test_overwrite() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config =
        AppConfig::<TestConfig>::from_custom_path(ConfigFileType::Toml, &config_dir).unwrap();

    std::fs::write(config_dir.join("config.toml"), "test = false").unwrap();
    assert!(!test_config.read_config().unwrap().load().test);
    test_config.overwrite_config_file().unwrap();
    assert!(test_config.read_config().unwrap().load().test);
}

#[test]
fn test_access() {
    let config_dir = tempdir().unwrap().into_path();
    let test_config =
        AppConfig::<TestConfig2>::from_custom_path(ConfigFileType::Toml, config_dir).unwrap();
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
