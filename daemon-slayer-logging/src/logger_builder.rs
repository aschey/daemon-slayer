use std::{
    env::args,
    io::stdout,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use directories::ProjectDirs;
use once_cell::sync::OnceCell;
use parity_tokio_ipc::{Connection, Endpoint};
use time::{
    format_description::well_known::{self, Rfc3339},
    UtcOffset,
};

use crate::ipc_writer::IpcFilter;

#[cfg(feature = "async-tokio")]
use super::ipc_writer::IpcWriter;
use super::{logger_guard::LoggerGuard, timezone::Timezone};
use tracing::metadata::LevelFilter;
use tracing_appender::{
    non_blocking::{NonBlockingBuilder, WorkerGuard},
    rolling::{RollingFileAppender, Rotation},
};
#[cfg(windows)]
use tracing_eventlog::{register, EventLogLayer};
use tracing_subscriber::{
    fmt::{time::OffsetTime, Layer, MakeWriter},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer as SubscriberLayer,
};

static LOCAL_TIME: OnceCell<Result<OffsetTime<Rfc3339>, time::error::IndeterminateOffset>> =
    OnceCell::new();

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
        LOCAL_TIME.get_or_init(OffsetTime::local_rfc_3339);
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
        let offset = match (self.timezone, LOCAL_TIME.get().unwrap().clone()) {
            (Timezone::Local, Ok(offset)) => offset,
            (Timezone::Local, Err(e)) => {
                println!("Error getting local time: {e}");
                OffsetTime::new(UtcOffset::UTC, well_known::Rfc3339)
            }
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
                    .with_timer(offset.clone())
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(non_blocking_stdout)
                    .with_filter(self.level_filter)
            })
            .with(tracing_error::ErrorLayer::default());

        let (ipc_writer, ipc_guard) = IpcWriter::new();
        guard.set_console_guard(ipc_guard);
        #[cfg(feature = "async-tokio")]
        let collector = collector.with({
            Layer::new()
                .compact()
                .with_timer(offset)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_writer(ipc_writer)
                .with_filter(IpcFilter::new(self.level_filter))
        });

        #[cfg(target_os = "linux")]
        let collector = collector.with(tracing_journald::layer().unwrap());

        #[cfg(windows)]
        register(&self.name).unwrap();
        #[cfg(windows)]
        let collector = collector.with(EventLogLayer::pretty(self.name).unwrap());

        (collector, guard)
    }
}
