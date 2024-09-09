use std::path::PathBuf;
use std::time::Duration;

use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::BroadcastEventStore;
use daemon_slayer_core::{BoxedError, FutureExt};
use notify::RecommendedWatcher;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use tap::TapFallible;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};

use super::file_watcher_client::FileWatcherClient;
use crate::file_watcher_builder::FileWatcherBuilder;
use crate::file_watcher_command::FileWatcherCommand;

pub struct FileWatcher {
    file_tx: broadcast::Sender<Vec<PathBuf>>,
    command_tx: mpsc::Sender<FileWatcherCommand>,
    command_rx: mpsc::Receiver<FileWatcherCommand>,
    debouncer: Debouncer<RecommendedWatcher>,
}

impl FileWatcher {
    pub fn builder() -> FileWatcherBuilder {
        FileWatcherBuilder::default()
    }

    pub(crate) fn from_builder(builder: FileWatcherBuilder) -> Self {
        let (file_tx, _) = broadcast::channel(32);
        let file_tx_ = file_tx.clone();

        let mut debouncer = new_debouncer(
            Duration::from_secs(builder.debounce_seconds),
            move |events: Result<Vec<DebouncedEvent>, notify::Error>| {
                if let Ok(events) = events.tap_err(|e| error!("File watch error: {e:?}")) {
                    let paths = events.into_iter().map(|e| e.path).collect();
                    file_tx_
                        .send(paths)
                        .tap_err(|e| warn!("Error sending file paths: {e:?}"))
                        .ok();
                }
            },
        )
        .unwrap();
        let watcher = debouncer.watcher();

        for path in builder.paths.iter() {
            match watcher.watch(path, notify::RecursiveMode::Recursive) {
                Ok(_) => {
                    info!("Watching {path:?}");
                }
                Err(e) => {
                    error!("Error watching {path:?}: {e:?}");
                }
            }
        }
        let (command_tx, command_rx) = mpsc::channel(32);
        Self {
            file_tx,
            command_tx,
            command_rx,
            debouncer,
        }
    }

    pub fn get_client(&self) -> FileWatcherClient {
        FileWatcherClient::new(self.command_tx.clone())
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<Vec<PathBuf>> {
        BroadcastEventStore::new(self.file_tx.clone())
    }
}

impl BackgroundService for FileWatcher {
    fn name(&self) -> &str {
        "file_watcher_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        while let Ok(Some(command)) = self
            .command_rx
            .recv()
            .cancel_with(context.cancelled())
            .await
        {
            match command {
                FileWatcherCommand::Watch(path, recursive_mode) => self
                    .debouncer
                    .watcher()
                    .watch(&path, recursive_mode)
                    .tap_err(|e| error!("Error watching path: {e:?}"))
                    .ok(),
                FileWatcherCommand::Unwatch(path) => self
                    .debouncer
                    .watcher()
                    .unwatch(&path)
                    .tap_err(|e| error!("Error watching path: {e:?}"))
                    .ok(),
            };
        }

        Ok(())
    }
}
