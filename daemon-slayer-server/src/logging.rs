use std::io::stdout;

use directories::ProjectDirs;
use time::{format_description::well_known, UtcOffset};
use tracing::metadata::LevelFilter;
use tracing_appender::{
    non_blocking::{NonBlockingBuilder, WorkerGuard},
    rolling::{RollingFileAppender, Rotation},
};
#[cfg(windows)]
use tracing_eventlog::{register, EventLogLayer};
use tracing_subscriber::{
    fmt::{time::OffsetTime, Layer},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer as SubscriberLayer,
};

pub struct LoggerGuard {
    guards: Vec<WorkerGuard>,
}

pub enum Timezone {
    Local,
    Utc,
}

impl LoggerGuard {
    fn new() -> Self {
        Self { guards: vec![] }
    }

    fn add_guard(&mut self, guard: WorkerGuard) {
        self.guards.push(guard);
    }
}

pub struct LoggerBuilder {
    name: String,
    file_rotation_period: Rotation,
    timezone: Timezone,
    output_buffer_limit: usize,
    default_log_level: tracing::Level,
    level_filter: LevelFilter,
}

impl LoggerBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            file_rotation_period: Rotation::HOURLY,
            timezone: Timezone::Local,
            // The default number of buffered lines is quite large and uses a ton of memory
            // We aren't logging a ton of messages so setting this value somewhat low is fine in order to conserve memory
            output_buffer_limit: 256,
            default_log_level: tracing::Level::INFO,
            level_filter: LevelFilter::INFO,
        }
    }

    pub fn with_file_rotation_period(mut self, rotation: Rotation) -> Self {
        self.file_rotation_period = rotation;
        self
    }

    pub fn with_timezone(mut self, timezone: Timezone) -> Self {
        self.timezone = timezone;
        self
    }

    pub fn with_output_buffer_limit(mut self, output_buffer_limit: usize) -> Self {
        self.output_buffer_limit = output_buffer_limit;
        self
    }

    pub fn with_default_log_level(mut self, default_log_level: tracing::Level) -> Self {
        self.default_log_level = default_log_level;
        self
    }

    pub fn build(self) -> (impl SubscriberInitExt, LoggerGuard) {
        let offset = match (self.timezone, OffsetTime::local_rfc_3339()) {
            (Timezone::Local, Ok(offset)) => offset,
            _ => OffsetTime::new(UtcOffset::UTC, well_known::Rfc3339),
        };

        let proj_dirs =
            ProjectDirs::from("", "", &self.name).expect("Unable to find a valid home directory");
        let log_dir = proj_dirs.cache_dir();
        let file_appender = RollingFileAppender::new(
            self.file_rotation_period,
            log_dir,
            format!("{}.log", self.name),
        );

        let mut guard = LoggerGuard::new();

        let (non_blocking_stdout, stdout_guard) = NonBlockingBuilder::default()
            .buffered_lines_limit(self.output_buffer_limit)
            .finish(stdout());
        guard.add_guard(stdout_guard);
        let (non_blocking_file, file_guard) = NonBlockingBuilder::default()
            .buffered_lines_limit(self.output_buffer_limit)
            .finish(file_appender);
        guard.add_guard(file_guard);
        let collector = tracing_subscriber::registry()
            .with(EnvFilter::from_default_env().add_directive(self.default_log_level.into()))
            .with({
                Layer::new()
                    .with_timer(offset.clone())
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_ansi(false)
                    .with_writer(non_blocking_file)
                    .with_filter(self.level_filter)
            })
            .with({
                Layer::new()
                    .pretty()
                    .with_timer(offset)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(non_blocking_stdout)
                    .with_filter(self.level_filter)
            })
            .with(tracing_error::ErrorLayer::default());

        #[cfg(windows)]
        register(&self.name).unwrap();
        #[cfg(windows)]
        let collector = collector.with(EventLogLayer::pretty(self.name).unwrap());

        (collector, guard)
    }
}
