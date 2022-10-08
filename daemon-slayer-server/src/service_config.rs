use std::path::PathBuf;

pub struct ServiceConfig {
    #[cfg(feature = "task-queue")]
    pub(crate) task_queue_builder: Option<daemon_slayer_task_queue::TaskQueueBuilder>,
    #[cfg(feature = "file-watcher")]
    pub(crate) watch_paths: Vec<PathBuf>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            task_queue_builder: Some(Default::default()),
            watch_paths: Default::default(),
        }
    }
}

impl ServiceConfig {
    #[cfg(feature = "task-queue")]
    pub fn with_task_queue_builder(
        mut self,
        task_queue_builder: daemon_slayer_task_queue::TaskQueueBuilder,
    ) -> Self {
        self.task_queue_builder = Some(task_queue_builder);
        self
    }

    #[cfg(feature = "task-queue")]
    pub fn with_job_handler<J>(mut self, job: J) -> Self
    where
        J: daemon_slayer_task_queue::JobProcessor + 'static,
        J::Payload: daemon_slayer_task_queue::Decode + daemon_slayer_task_queue::Encode,
        J::Error: Into<daemon_slayer_task_queue::JobError>,
    {
        if let Some(q) = self.task_queue_builder.take() {
            self.task_queue_builder = Some(q.with_job_handler(job));
        }
        self
    }

    #[cfg(feature = "task-queue")]
    pub fn with_task_queue_database_path(mut self, path: impl AsRef<std::path::Path>) -> Self {
        if let Some(q) = self.task_queue_builder.take() {
            let options = daemon_slayer_task_queue::SqliteConnectOptions::default().filename(path);
            let options =
                daemon_slayer_task_queue::TaskQueueBuilder::apply_default_sqlite_options(options);
            self.task_queue_builder = Some(q.with_sqlite_options(options));
        }
        self
    }

    #[cfg(feature = "task-queue")]
    pub fn with_task_queue_concurrency(mut self, concurrency: usize) -> Self {
        if let Some(q) = self.task_queue_builder.take() {
            self.task_queue_builder = Some(q.with_concurrency(concurrency));
        }
        self
    }

    #[cfg(feature = "file-watcher")]
    pub fn with_watch_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.watch_paths.push(path.into());
        self
    }
}
