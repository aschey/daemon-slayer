use std::path::PathBuf;

pub struct FileWatcherBuilder {
    pub(crate) debounce_seconds: u64,
    pub(crate) paths: Vec<PathBuf>,
}
