use std::{
    error::Error,
    io::{stderr, stdout},
};

use once_cell::sync::OnceCell;
use time::{
    format_description::well_known::{self, Rfc3339},
    UtcOffset,
};

use super::{logger_guard::LoggerGuard, timezone::Timezone};
use tracing::{metadata::LevelFilter, Subscriber};
use tracing_appender::non_blocking::NonBlockingBuilder;

use tracing_subscriber::{
    fmt::{time::OffsetTime, Layer},
    prelude::*,
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter, Layer as SubscriberLayer,
};

static LOCAL_TIME: OnceCell<Result<OffsetTime<Rfc3339>, time::error::IndeterminateOffset>> =
    OnceCell::new();

pub fn init_local_time() {
    LOCAL_TIME.get_or_init(OffsetTime::local_rfc_3339);
}

pub struct LoggerBuilder {
    name: String,
    #[cfg(feature = "file")]
    file_rotation_period: tracing_appender::Rotation,
    timezone: Timezone,
    output_buffer_limit: usize,
    default_log_level: tracing::Level,
    level_filter: LevelFilter,
    log_to_stdout: bool,
    log_to_stderr: bool,
    #[cfg(feature = "async-tokio")]
    enable_ipc_logger: bool,
}

impl LoggerBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            #[cfg(feature = "file")]
            file_rotation_period: tracing_appender::Rotation::HOURLY,
            timezone: Timezone::Local,
            // The default number of buffered lines is quite large and uses a ton of memory
            // We aren't logging a ton of messages so setting this value somewhat low is fine in order to conserve memory
            output_buffer_limit: 256,
            default_log_level: tracing::Level::INFO,
            level_filter: LevelFilter::INFO,
            log_to_stdout: false,
            log_to_stderr: true,
            #[cfg(feature = "async-tokio")]
            enable_ipc_logger: false,
        }
    }

    #[cfg(feature = "file")]
    pub fn with_file_rotation_period(mut self, rotation: tracing_appender::Rotation) -> Self {
        self.file_rotation_period = rotation;
        self
    }

    pub fn with_timezone(mut self, timezone: Timezone) -> Self {
        self.timezone = timezone;
        self
    }

    pub fn with_log_to_stdout(mut self, log_to_stdout: bool) -> Self {
        self.log_to_stdout = log_to_stdout;
        self
    }

    pub fn with_log_to_stderr(mut self, log_to_stderr: bool) -> Self {
        self.log_to_stderr = log_to_stderr;
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

    pub fn with_level_filter(mut self, level_filter: LevelFilter) -> Self {
        self.level_filter = level_filter;
        self
    }

    #[cfg(feature = "async-tokio")]
    pub fn with_ipc_logger(mut self, enable_ipc_logger: bool) -> Self {
        self.enable_ipc_logger = enable_ipc_logger;
        self
    }

    pub fn register(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        #[cfg(all(windows, feature = "windows-eventlog"))]
        {
            use tracing_eventlog::EventLogRegistry;
            let log_source = tracing_eventlog::LogSource::application(self.name.clone());
            log_source.register()?;
        }
        Ok(())
    }

    pub fn deregister(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        #[cfg(all(windows, feature = "windows-eventlog"))]
        {
            use tracing_eventlog::EventLogRegistry;
            let log_source = tracing_eventlog::LogSource::application(self.name.clone());
            log_source.deregister()?;
        }
        Ok(())
    }

    pub fn build(
        self,
    ) -> Result<
        (
            impl SubscriberInitExt + Subscriber + for<'a> LookupSpan<'a>,
            LoggerGuard,
        ),
        Box<dyn Error + Send + Sync>,
    > {
        let offset = match (self.timezone, LOCAL_TIME.get()) {
            (Timezone::Local, Some(Ok(offset))) => offset.to_owned(),
            (Timezone::Local, Some(Err(e))) => {
                println!("Error getting local time: {e}");
                OffsetTime::new(UtcOffset::UTC, well_known::Rfc3339)
            }
            _ => OffsetTime::new(UtcOffset::UTC, well_known::Rfc3339),
        };

        let collector = tracing_subscriber::registry()
            .with(EnvFilter::from_default_env().add_directive(self.default_log_level.into()));

        let mut guard = LoggerGuard::new();

        #[cfg(feature = "file")]
        let proj_dirs = directories::ProjectDirs::from("", "", &self.name)
            .expect("Unable to find a valid home directory");
        #[cfg(feature = "file")]
        let log_dir = proj_dirs.cache_dir();
        #[cfg(feature = "file")]
        let file_appender = tracing_appender::rolling::RollingFileAppender::new(
            self.file_rotation_period,
            log_dir,
            format!("{}.log", self.name),
        );
        #[cfg(feature = "file")]
        let (non_blocking_file, file_guard) = NonBlockingBuilder::default()
            .buffered_lines_limit(self.output_buffer_limit)
            .finish(file_appender);
        #[cfg(feature = "file")]
        guard.add_guard(Box::new(file_guard));
        #[cfg(feature = "file")]
        let collector = collector.with({
            Layer::new()
                .with_timer(offset.clone())
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_ansi(false)
                .with_writer(non_blocking_file)
                .with_filter(self.level_filter)
        });

        let (non_blocking_stdout, stdout_guard) = NonBlockingBuilder::default()
            .buffered_lines_limit(self.output_buffer_limit)
            .finish(stdout());
        guard.add_guard(Box::new(stdout_guard));

        let (non_blocking_stderr, stderr_guard) = NonBlockingBuilder::default()
            .buffered_lines_limit(self.output_buffer_limit)
            .finish(stderr());
        guard.add_guard(Box::new(stderr_guard));

        let collector = collector
            .with({
                Layer::new()
                    .pretty()
                    .with_timer(offset.clone())
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(non_blocking_stdout)
                    .with_filter(if self.log_to_stdout {
                        self.level_filter
                    } else {
                        LevelFilter::OFF
                    })
            })
            .with({
                Layer::new()
                    .pretty()
                    .with_timer(offset.clone())
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(non_blocking_stderr)
                    .with_filter(if self.log_to_stderr {
                        self.level_filter
                    } else {
                        LevelFilter::OFF
                    })
            })
            .with(tracing_error::ErrorLayer::default());

        #[cfg(feature = "ipc")]
        let (ipc_writer, ipc_guard) = if self.enable_ipc_logger {
            tracing_ipc::Writer::new(&self.name)
        } else {
            tracing_ipc::Writer::disabled()
        };
        #[cfg(feature = "ipc")]
        guard.add_guard(Box::new(ipc_guard));
        #[cfg(feature = "ipc")]
        let collector = collector.with({
            Layer::new()
                .compact()
                .with_timer(offset)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_writer(ipc_writer)
                .with_filter(tracing_ipc::Filter::new(self.level_filter))
        });

        #[cfg(all(target_os = "linux", feature = "journald"))]
        let collector = collector.with(tracing_journald::layer()?);

        #[cfg(all(windows, feature = "eventlog"))]
        let collector = collector.with(tracing_eventlog::EventLogLayer::pretty(self.name)?);

        Ok((collector, guard))
    }
}
