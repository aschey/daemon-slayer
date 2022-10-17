use aide_de_camp::prelude::JobProcessor;
use aide_de_camp::prelude::{Decode, Encode, JobError, RunnerRouter};
use aide_de_camp_sqlite::sqlx::sqlite::SqliteConnectOptions;
use aide_de_camp_sqlite::sqlx::ConnectOptions;
use log::LevelFilter;
use std::path::Path;
use std::str::FromStr;

use crate::TaskQueue;

pub struct TaskQueueBuilder {
    pub(crate) sqlite_options: SqliteConnectOptions,
    pub(crate) router: RunnerRouter,
    pub(crate) concurrency: usize,
}

impl Default for TaskQueueBuilder {
    fn default() -> Self {
        Self::from_base_sqlite_options(SqliteConnectOptions::from_str(":memory:").unwrap())
    }
}

impl TaskQueueBuilder {
    pub fn from_database_path(path: impl AsRef<Path>) -> Self {
        Self::from_base_sqlite_options(SqliteConnectOptions::default().filename(path))
    }

    fn from_base_sqlite_options(options: SqliteConnectOptions) -> Self {
        Self {
            sqlite_options: Self::apply_default_sqlite_options(options),
            router: RunnerRouter::default(),
            concurrency: 10,
        }
    }

    pub fn apply_default_sqlite_options(options: SqliteConnectOptions) -> SqliteConnectOptions {
        options
            .create_if_missing(true)
            .log_statements(LevelFilter::Debug)
            .log_slow_statements(LevelFilter::Info, std::time::Duration::from_secs(1))
            .to_owned()
    }

    pub fn with_sqlite_options(mut self, options: SqliteConnectOptions) -> Self {
        self.sqlite_options = options;
        self
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn with_job_handler<J>(mut self, job: J) -> Self
    where
        J: JobProcessor + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        self.router.add_job_handler(job);
        self
    }

    pub async fn start(self) -> TaskQueue {
        TaskQueue::from_builder(self).await
    }
}
