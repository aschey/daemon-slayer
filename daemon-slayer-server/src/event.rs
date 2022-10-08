use crate::Signal;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    SignalReceived(Signal),
    #[cfg(feature = "file-watcher")]
    FileChanged(Vec<std::path::PathBuf>),
    #[cfg(feature = "task-queue")]
    TaskQueueEvent(daemon_slayer_task_queue::JobEvent),
}
