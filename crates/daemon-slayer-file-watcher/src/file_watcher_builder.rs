use std::path::PathBuf;

use crate::FileWatcher;

#[derive(Clone)]
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

    pub fn with_watch_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.paths.push(path.into());
        self
    }

    pub fn with_watch_paths(mut self, paths: Vec<impl Into<PathBuf>>) -> Self {
        let mut paths = paths.into_iter().map(Into::into).collect();
        self.paths.append(&mut paths);
        self
    }

    pub fn build(self) -> FileWatcher {
        FileWatcher::from_builder(self)
    }
}
