use crate::Signal;

#[derive(Debug, Clone)]
pub enum Event {
    SignalReceived(Signal),
    #[cfg(feature = "file-watcher")]
    FileChanged(Vec<std::path::PathBuf>),
}
