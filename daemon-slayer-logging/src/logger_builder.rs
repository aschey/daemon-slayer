use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display},
    io::{stderr, stdout},
    ops::Deref,
    str::FromStr,
};

use daemon_slayer_core::{
    config::{Accessor, CachedConfig},
    server::BackgroundService,
};
use once_cell::sync::OnceCell;
use serde::de::Visitor;
use time::{
    format_description::well_known::{self, Rfc3339},
    UtcOffset,
};

use super::{logger_guard::LoggerGuard, timezone::Timezone};
use tracing::{
    metadata::{LevelFilter, ParseLevelError},
    Level, Subscriber,
};
use tracing_appender::non_blocking::NonBlockingBuilder;

use tracing_subscriber::{
    filter::Directive,
    fmt::{time::OffsetTime, Layer},
    prelude::*,
    registry::LookupSpan,
    reload::{self, Handle},
    util::SubscriberInitExt,
    EnvFilter, Layer as SubscriberLayer,
};

static LOCAL_TIME: OnceCell<Result<OffsetTime<Rfc3339>, time::error::IndeterminateOffset>> =
    OnceCell::new();

pub fn init_local_time() {
    LOCAL_TIME.get_or_init(OffsetTime::local_rfc_3339);
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum LogTarget {
    File,
    EventLog,
    JournalD,
    OsLog,
    Stdout,
    Stderr,
    Ipc,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogLevel(pub(crate) Level);

impl LogLevel {
    pub(crate) fn to_level_filter(&self) -> LevelFilter {
        LevelFilter::from_level(self.0)
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel(Level::INFO)
    }
}

impl Deref for LogLevel {
    type Target = Level;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "config")]
impl<'de> serde::Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val = String::deserialize(deserializer)?;
        let level = Level::from_str(&val).map_err(serde::de::Error::custom)?;
        Ok(LogLevel(level))
    }
}

#[derive(daemon_slayer_core::Mergeable, Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "config", derive(confique::Config, serde::Deserialize))]
pub struct UserConfig {
    #[cfg_attr(feature = "config", config(default = "info"))]
    pub(crate) log_level: LogLevel,
}

#[derive(Clone)]
pub struct LoggerBuilder {
    name: String,
    #[cfg(feature = "file")]
    file_rotation_period: tracing_appender::Rotation,
    timezone: Timezone,
    output_buffer_limit: usize,
    user_config: CachedConfig<UserConfig>,
    target_directives: HashMap<LogTarget, Vec<Directive>>,
    env_filter_directives: Vec<Directive>,
    log_to_stdout: bool,
    log_to_stderr: bool,
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
            user_config: Default::default(),
            log_to_stdout: false,
            log_to_stderr: true,
            enable_ipc_logger: false,
            target_directives: Default::default(),
            env_filter_directives: vec![],
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

    pub fn with_config<S>(mut self, service: S) -> Self
    where
        S: Accessor<UserConfig> + Clone + Unpin + 'static,
    {
        self.user_config = service.access();
        self
    }

    pub fn with_env_filter_directive(mut self, directive: Directive) -> Self {
        self.env_filter_directives.push(directive);
        self
    }

    pub fn with_ipc_logger(mut self, enable_ipc_logger: bool) -> Self {
        self.enable_ipc_logger = enable_ipc_logger;
        self
    }

    pub fn with_target_directive(mut self, target: LogTarget, directive: Directive) -> Self {
        if let Some(directives) = self.target_directives.get_mut(&target) {
            directives.push(directive);
        } else {
            self.target_directives.insert(target, vec![directive]);
        }
        self
    }

    fn get_filter_for_target(&self, target: LogTarget) -> EnvFilter {
        if let Some(directives) = self.target_directives.get(&target) {
            let mut env_filter = EnvFilter::from_default_env()
                .add_directive(self.user_config.snapshot().log_level.0.into());
            for directive in directives {
                env_filter = env_filter.add_directive(directive.clone());
            }
            env_filter
        } else {
            EnvFilter::from_default_env()
                .add_directive(self.user_config.snapshot().log_level.0.into())
        }
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
            log_source.deregister().ok();
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
        let offset = match (&self.timezone, LOCAL_TIME.get()) {
            (Timezone::Local, Some(Ok(offset))) => offset.to_owned(),
            (Timezone::Local, Some(Err(e))) => {
                println!("Error getting local time: {e}");
                OffsetTime::new(UtcOffset::UTC, well_known::Rfc3339)
            }
            _ => OffsetTime::new(UtcOffset::UTC, well_known::Rfc3339),
        };

        let mut env_filter = EnvFilter::from_default_env()
            .add_directive(self.user_config.snapshot().log_level.0.into());
        for directive in &self.env_filter_directives {
            env_filter = env_filter.add_directive(directive.clone());
        }

        let collector = tracing_subscriber::registry().with(env_filter);

        let mut guard = LoggerGuard::default();

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
                        self.get_filter_for_target(LogTarget::Stdout)
                    } else {
                        EnvFilter::from_default_env().add_directive(LevelFilter::OFF.into())
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
                        self.get_filter_for_target(LogTarget::Stderr)
                    } else {
                        EnvFilter::from_default_env().add_directive(LevelFilter::OFF.into())
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
                .with_filter(tracing_ipc::Filter::new(
                    self.get_filter_for_target(LogTarget::Ipc),
                ))
        });

        #[cfg(all(target_os = "linux", feature = "linux-journald"))]
        let collector = collector.with(
            tracing_journald::layer()?.with_filter(self.get_filter_for_target(LogTarget::JournalD)),
        );

        #[cfg(all(target_os = "macos", feature = "mac-oslog"))]
        let collector = collector.with(
            tracing_oslog::OsLogger::new(self.name.clone(), "default")
                .with_filter(self.get_filter_for_target(LogTarget::OsLog)),
        );

        #[cfg(all(windows, feature = "windows-eventlog"))]
        let collector = collector.with(
            tracing_eventlog::EventLogLayer::pretty(self.name.clone())?
                .with_filter(self.get_filter_for_target(LogTarget::EventLog)),
        );

        let (filter, reload_handle) =
            reload::Layer::new(self.user_config.snapshot().log_level.0.into());
        guard.set_reload_handle(Box::new(move |level_filter| {
            reload_handle.modify(|l| *l = level_filter).unwrap();
        }));

        let collector = collector.with(filter);

        Ok((collector, guard))
    }
}
