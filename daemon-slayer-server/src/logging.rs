use std::{
    env::args,
    io::stdout,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use directories::ProjectDirs;
use once_cell::sync::OnceCell;
use parity_tokio_ipc::{Connection, Endpoint};
use time::{format_description::well_known, UtcOffset};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
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

#[derive(Debug)]
enum IpcCmd {
    Flush,
    Write(Vec<u8>),
}

struct IpcWriterInstance {
    tx: tokio::sync::mpsc::Sender<IpcCmd>,
}

impl std::io::Write for IpcWriterInstance {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if !IS_INITIALIZED.load(Ordering::SeqCst) {
            return Ok(buf.len());
        }
        let b = buf.to_owned();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            if let Err(e) = tx.send(IpcCmd::Write(b)).await {
                println!("IpcWriterInstance Err writing {e}");
            }
        });

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !IS_INITIALIZED.load(Ordering::SeqCst) {
            return Ok(());
        }
        let tx = self.tx.clone();

        tokio::spawn(async move {
            if let Err(e) = tx.send(IpcCmd::Flush).await {
                println!("IpcWriterInstance Err flushing {e:?}");
            }
        });

        Ok(())
    }
}

struct IpcWriter;

static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);
static SENDER: OnceCell<tokio::sync::mpsc::Sender<IpcCmd>> = OnceCell::new();

impl IpcWriter {
    fn new() -> Self {
        Self
    }

    fn init(&self, mut rx: tokio::sync::mpsc::Receiver<IpcCmd>) {
        tokio::spawn(async move {
            let mut client = loop {
                match Endpoint::connect("/tmp/daemon_slayer.sock").await {
                    Ok(client) => break client,
                    Err(e) => {
                        println!("Error connecting {e:?}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            };

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    IpcCmd::Write(buf) => {
                        client
                            .write_all(&buf)
                            .await
                            .expect("Unable to write message to client");
                    }
                    IpcCmd::Flush => {
                        client.flush().await.unwrap();
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        });
    }
}

impl MakeWriter<'_> for IpcWriter {
    type Writer = IpcWriterInstance;

    fn make_writer(&'_ self) -> Self::Writer {
        let is_console = args().len() > 1 && args().skip(1).take(1).next().unwrap() == "console";
        if !IS_INITIALIZED.swap(!is_console, Ordering::SeqCst) {
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            SENDER.get_or_init(|| tx);

            if !is_console {
                self.init(rx);
            }
        }

        IpcWriterInstance {
            tx: SENDER.get().unwrap().clone(),
        }
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
            .with({
                Layer::new()
                    .compact()
                    .with_timer(offset)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(IpcWriter::new())
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
