use std::collections::HashMap;
use std::io::{self, stderr, stdout};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

use daemon_slayer_core::config::Accessor;
use daemon_slayer_core::{BoxedError, Label, Mergeable};
use time::UtcOffset;
use time::format_description::well_known::{self, Rfc3339};
use tracing::metadata::LevelFilter;
use tracing::{Level, Subscriber, debug};
use tracing_appender::non_blocking::NonBlockingBuilder;
use tracing_subscriber::filter::Directive;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer as SubscriberLayer, reload};

use super::logger_guard::LoggerGuard;
use super::timezone::Timezone;
use crate::ReloadHandle;

static LOGGER_GUARD: OnceLock<Mutex<Option<LoggerGuard>>> = OnceLock::new();

static LOCAL_TIME: OnceLock<Result<OffsetTime<Rfc3339>, time::error::IndeterminateOffset>> =
    OnceLock::new();

#[must_use]
pub struct GlobalLoggerGuard;

impl Drop for GlobalLoggerGuard {
    fn drop(&mut self) {
        debug!("Dropping global logger guard");
        LOGGER_GUARD.get().unwrap().lock().unwrap().take();
    }
}

#[ctor::ctor]
fn init_time() {
    LOCAL_TIME.set(OffsetTime::local_rfc_3339()).ok();
}

pub fn init() -> GlobalLoggerGuard {
    GlobalLoggerGuard
}

#[derive(thiserror::Error, Debug)]
pub enum LoggerCreationError {
    #[cfg(feature = "linux-journald")]
    #[error("Error creating journald logging layer: {0}")]
    JournaldFailure(io::Error),
    #[cfg(feature = "windows-eventlog")]
    #[error("Error creating event log layer: {0}")]
    EventLogError(String),
    #[cfg(feature = "file")]
    #[error("Error creating file logging layer: Unable to locate a home directory")]
    NoHomeDir,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
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
pub struct LogLevel(pub Level);

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

#[derive(Debug, Clone, Default, PartialEq, Eq, Mergeable)]
#[cfg_attr(feature = "config", derive(confique::Config, serde::Deserialize))]
pub struct UserConfig {
    #[cfg_attr(feature = "config", config(default = "info"))]
    pub log_level: LogLevel,
}

#[derive(Debug, Clone)]
pub struct EnvConfig {
    var_name: String,
    default_if_missing: LevelFilter,
}

impl EnvConfig {
    pub fn new(var_name: String) -> Self {
        Self {
            var_name,
            default_if_missing: LevelFilter::ERROR,
        }
    }

    pub fn with_default(mut self, default: LevelFilter) -> Self {
        self.default_if_missing = default;
        self
    }
}

#[derive(Debug, Clone)]
pub struct LoggerBuilder {
    #[allow(dead_code)]
    label: Label,
    #[cfg(feature = "file")]
    file_rotation_period: tracing_appender::rolling::Rotation,
    timezone: Timezone,
    output_buffer_limit: usize,
    target_directives: HashMap<LogTarget, Vec<Directive>>,
    env_filter_directives: Vec<Directive>,
    log_to_stdout: bool,
    log_to_stderr: bool,
    #[cfg(feature = "ipc")]
    enable_ipc_logger: bool,
    env_config: Option<EnvConfig>,
}

impl LoggerBuilder {
    pub fn new(label: Label) -> Self {
        Self {
            label,
            #[cfg(feature = "file")]
            file_rotation_period: tracing_appender::rolling::Rotation::HOURLY,
            timezone: Timezone::Local,
            // The default number of buffered lines is quite large and uses a ton of memory
            // We aren't logging a ton of messages so setting this value somewhat low is fine in
            // order to conserve memory
            output_buffer_limit: 256,
            log_to_stdout: false,
            log_to_stderr: true,
            #[cfg(feature = "ipc")]
            enable_ipc_logger: false,
            target_directives: Default::default(),
            env_filter_directives: vec![],
            env_config: None,
        }
    }

    #[cfg(feature = "file")]
    pub fn with_file_rotation_period(
        mut self,
        rotation: tracing_appender::rolling::Rotation,
    ) -> Self {
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

    pub fn with_env_filter_directive(mut self, directive: Directive) -> Self {
        self.env_filter_directives.push(directive);
        self
    }

    #[cfg(feature = "ipc")]
    pub fn with_ipc_logger(mut self, enable_ipc_logger: bool) -> Self {
        self.enable_ipc_logger = enable_ipc_logger;
        self
    }

    pub fn with_env_config(mut self, config: EnvConfig) -> Self {
        self.env_config = Some(config);
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

    fn get_base_env_filter(&self) -> EnvFilter {
        if let Some(config) = &self.env_config {
            EnvFilter::try_from_env(config.var_name.clone()).unwrap_or_else(|_| {
                EnvFilter::default().add_directive(config.default_if_missing.into())
            })
        } else {
            EnvFilter::default()
        }
    }

    fn get_filter_for_target(&self, target: LogTarget) -> EnvFilter {
        let mut env_filter = self.get_base_env_filter();
        for directive in &self.env_filter_directives {
            env_filter = env_filter.add_directive(directive.clone());
        }
        if let Some(directives) = self.target_directives.get(&target) {
            for directive in directives {
                env_filter = env_filter.add_directive(directive.clone());
            }
        }
        env_filter
    }

    pub fn register(&self) -> Result<(), BoxedError> {
        #[cfg(all(windows, feature = "windows-eventlog"))]
        {
            use tracing_eventlog::EventLogRegistry;
            let log_source = tracing_eventlog::LogSource::application(&self.label.application);
            log_source.register()?;
        }
        Ok(())
    }

    pub fn deregister(&self) -> Result<(), BoxedError> {
        #[cfg(all(windows, feature = "windows-eventlog"))]
        {
            use tracing_eventlog::EventLogRegistry;
            let log_source = tracing_eventlog::LogSource::application(&self.label.application);
            log_source.deregister().ok();
        }
        Ok(())
    }

    pub fn build(
        self,
    ) -> Result<impl SubscriberInitExt + Subscriber + for<'a> LookupSpan<'a>, LoggerCreationError>
    {
        let offset = match (&self.timezone, LOCAL_TIME.get()) {
            (Timezone::Local, Some(Ok(offset))) => offset.to_owned(),
            (Timezone::Local, Some(Err(e))) => {
                println!("Error getting local time: {e}");
                OffsetTime::new(UtcOffset::UTC, well_known::Rfc3339)
            }
            _ => OffsetTime::new(UtcOffset::UTC, well_known::Rfc3339),
        };

        let mut env_filter = self.get_base_env_filter();
        for directive in &self.env_filter_directives {
            env_filter = env_filter.add_directive(directive.clone());
        }

        let collector = tracing_subscriber::registry().with(env_filter);

        let mut guard = LoggerGuard::default();

        #[cfg(feature = "file")]
        let proj_dirs = directories::ProjectDirs::from(
            &self.label.qualifier,
            &self.label.organization,
            &self.label.application,
        )
        .ok_or(LoggerCreationError::NoHomeDir)?;
        #[cfg(feature = "file")]
        let log_dir = proj_dirs.cache_dir();
        #[cfg(feature = "file")]
        let file_appender = tracing_appender::rolling::RollingFileAppender::new(
            self.file_rotation_period.clone(),
            log_dir,
            format!("{}.log", self.label.application),
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
                .with_filter(self.get_filter_for_target(LogTarget::File))
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
                        EnvFilter::default().add_directive(LevelFilter::OFF.into())
                    })
            })
            .with({
                #[allow(clippy::redundant_clone)]
                Layer::new()
                    .pretty()
                    .with_timer(offset.clone())
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(non_blocking_stderr)
                    .with_filter(if self.log_to_stderr {
                        self.get_filter_for_target(LogTarget::Stderr)
                    } else {
                        EnvFilter::default().add_directive(LevelFilter::OFF.into())
                    })
            })
            .with(tracing_error::ErrorLayer::default());

        #[cfg(feature = "ipc")]
        let (ipc_writer, ipc_guard) = {
            use tilia::transport_async::Bind;
            use tilia::transport_async::codec::{CodecStream, LengthDelimitedCodec};
            use tilia::transport_async::ipc::{self, ServerId};
            let name = self.label.application.to_owned() + "_logger";
            let make_transport = move || {
                let name = name.to_owned();
                Box::pin(async move {
                    let transport = ipc::Endpoint::bind(
                        ipc::EndpointParams::new(
                            ServerId::new(name),
                            ipc::SecurityAttributes::allow_everyone_create().unwrap(),
                            ipc::OnConflict::Overwrite,
                        )
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                    CodecStream::new(transport, LengthDelimitedCodec)
                })
            };
            if self.enable_ipc_logger {
                let (writer, guard) = tilia::Writer::new(1024, make_transport);
                // Attempt to eagerly initialize the RPC server
                // since this function is likely called from the main thread
                // where we should have a Tokio runtime available

                // Without this, logging may not work properly from within
                // the windows service handler
                writer.init();
                (writer, guard)
            } else {
                tilia::Writer::disabled(make_transport)
            }
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
                .with_filter(tilia::Filter::new(
                    self.get_filter_for_target(LogTarget::Ipc),
                ))
        });

        #[cfg(all(target_os = "linux", feature = "linux-journald"))]
        let collector = collector.with(
            tracing_journald::layer()
                .map_err(LoggerCreationError::JournaldFailure)?
                .with_filter(self.get_filter_for_target(LogTarget::JournalD)),
        );

        #[cfg(all(target_os = "macos", feature = "mac-oslog"))]
        let collector = collector.with(
            tracing_oslog::OsLogger::new(self.label.qualified_name(), "default")
                .with_filter(self.get_filter_for_target(LogTarget::OsLog)),
        );

        #[cfg(all(windows, feature = "windows-eventlog"))]
        let collector = collector.with(
            tracing_eventlog::EventLogLayer::pretty(self.label.application.clone())
                .map_err(|e| LoggerCreationError::EventLogError(e.to_string()))?
                .with_filter(self.get_filter_for_target(LogTarget::EventLog)),
        );

        LOGGER_GUARD.set(Mutex::new(Some(guard))).ok();
        Ok(collector)
    }

    pub fn build_with_reload<S>(
        self,
        service: S,
    ) -> Result<
        (
            impl SubscriberInitExt + Subscriber + for<'a> LookupSpan<'a>,
            ReloadHandle,
        ),
        LoggerCreationError,
    >
    where
        S: Accessor<UserConfig> + Clone + Unpin + 'static,
    {
        let user_config = service.access();
        let collector = self.build()?;
        let (filter, reload_handle) = reload::Layer::new(user_config.snapshot().log_level.0.into());
        let reload_fn = Box::new(move |level_filter: LevelFilter| {
            reload_handle.modify(|l| *l = level_filter).ok();
        });

        let collector = collector.with(filter);
        Ok((collector, ReloadHandle::new(reload_fn)))
    }
}
