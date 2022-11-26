use std::{
    env,
    fs::File,
    io::Write,
    marker::PhantomData,
    path::{Path, PathBuf},
    process::Command,
};

use bat::{Input, PagingMode, PrettyPrinter};
use confique::{json5, toml, yaml, Config, FormatOptions};
use daemon_slayer_core::App;
use directories::ProjectDirs;
#[cfg(feature = "cli")]
pub mod cli;

pub enum ConfigFileType {
    Toml,
    Yaml,
    Json5,
}

impl ConfigFileType {
    fn to_extension(&self) -> &str {
        match &self {
            ConfigFileType::Toml => ".toml",
            ConfigFileType::Yaml => ".yaml",
            ConfigFileType::Json5 => ".json5",
        }
    }

    fn to_format_language(&self) -> &str {
        match &self {
            ConfigFileType::Toml => "toml",
            ConfigFileType::Yaml => "yaml",
            ConfigFileType::Json5 => "javascript",
        }
    }
}

pub struct AppConfig<T: Config> {
    config_file_type: ConfigFileType,
    phantom: PhantomData<T>,
    config_path: PathBuf,
}

impl<T: Config> AppConfig<T> {
    pub fn new(app: App, config_file_type: ConfigFileType) -> Self {
        let dirs = ProjectDirs::from(&app.qualifier, &app.organization, &app.application).unwrap();
        let config_path = dirs
            .config_dir()
            .join(format!("config{}", config_file_type.to_extension()));
        Self {
            config_file_type,
            config_path,
            phantom: Default::default(),
        }
    }

    pub fn config_template(&self) -> String {
        match self.config_file_type {
            ConfigFileType::Yaml => yaml::template::<T>(yaml::FormatOptions::default()),
            ConfigFileType::Toml => toml::template::<T>(toml::FormatOptions::default()),
            ConfigFileType::Json5 => json5::template::<T>(json5::FormatOptions::default()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.config_path
    }

    pub fn edit(&self) {
        if cfg!(unix) {
            let editor = match env::var("VISUAL").or_else(|_| env::var("EDITOR")) {
                Ok(editor) => editor,
                Err(_) => match which::which("vim")
                    .or_else(|_| which::which("emacs"))
                    .or_else(|_| which::which("nano"))
                    .or_else(|_| which::which("pico"))
                    .or_else(|_| which::which("vi"))
                {
                    Ok(path) => path.file_name().unwrap().to_string_lossy().to_string(),
                    Err(_) => "".to_string(),
                },
            };
            Command::new(editor)
                .arg(self.path())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        }
    }

    pub fn pretty_print(&self) {
        PrettyPrinter::new()
            .input_file(&self.config_path)
            .grid(true)
            .header(true)
            .paging_mode(PagingMode::QuitIfOneScreen)
            .line_numbers(true)
            .language(self.config_file_type.to_format_language())
            .print()
            .unwrap();
    }

    pub fn create_config_template(&self) {
        if self.config_path.exists() {
            return;
        }

        std::fs::create_dir_all(
            self.config_path
                .parent()
                .expect("Path should have a parent directory"),
        )
        .unwrap();
        let mut file = File::create(&self.config_path).unwrap();
        file.write_all(self.config_template().as_bytes()).unwrap();
    }

    pub fn read_config(&self) -> T {
        T::builder().env().file(&self.config_path).load().unwrap()
    }
}
