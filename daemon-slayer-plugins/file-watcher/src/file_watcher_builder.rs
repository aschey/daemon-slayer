use std::path::PathBuf;

pub struct FileWatcherBuilder {
    pub(crate) debounce_seconds: u64,
    pub(crate) paths: Vec<PathBuf>,
}

impl Default for FileWatcherBuilder {
    fn default() -> Self {
        Self {
            debounce_seconds: 2,
            paths: Default::default(),
        }
    }
}

impl FileWatcherBuilder {
    pub fn with_debounce_seconds(mut self, debounce_seconds: u64) -> Self {
        self.debounce_seconds = debounce_seconds;
        self
    }

    pub fn with_watch_path(mut self, path: PathBuf) -> Self {
        self.paths.push(path);
        self
    }

    pub fn with_watch_paths(mut self, mut paths: Vec<PathBuf>) -> Self {
        self.paths.append(&mut paths);
        self
    }
}
