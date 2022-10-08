use std::path::PathBuf;

#[derive(Default)]
pub struct ServiceConfig {
    #[cfg(feature = "task-queue")]
    pub(crate) router: Option<daemon_slayer_task_queue::RunnerRouter>,
    #[cfg(feature = "file-watcher")]
    pub(crate) watch_paths: Vec<PathBuf>,
}

impl ServiceConfig {
    #[cfg(feature = "task-queue")]
    pub fn with_job_handler<J>(mut self, job: J) -> Self
    where
        J: daemon_slayer_task_queue::JobProcessor + 'static,
        J::Payload: daemon_slayer_task_queue::Decode + daemon_slayer_task_queue::Encode,
        J::Error: Into<daemon_slayer_task_queue::JobError>,
    {
        if self.router.is_none() {
            self.router = Some(daemon_slayer_task_queue::RunnerRouter::default());
        }
        if let Some(r) = self.router.as_mut() {
            r.add_job_handler(job)
        }
        self
    }

    #[cfg(feature = "file-watcher")]
    pub fn with_watch_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.watch_paths.push(path.into());
        self
    }
}
