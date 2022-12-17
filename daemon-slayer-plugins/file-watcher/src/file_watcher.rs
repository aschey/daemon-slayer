use std::{
    error::Error,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use daemon_slayer_core::server::{BroadcastEventStore, FutureExt, ServiceContext, SubsystemHandle};
use notify::RecommendedWatcher;
use notify_debouncer_mini::Debouncer;
use tracing::{error, info};

use crate::{file_watcher_builder::FileWatcherBuilder, file_watcher_command::FileWatcherCommand};

use super::file_watcher_client::FileWatcherClient;

pub struct FileWatcher {
    file_tx: tokio::sync::broadcast::Sender<Vec<PathBuf>>,
    command_tx: tokio::sync::mpsc::Sender<FileWatcherCommand>,
    command_rx: tokio::sync::mpsc::Receiver<FileWatcherCommand>,
    debouncer: Debouncer<RecommendedWatcher>,
}

impl FileWatcher {
    pub fn builder() -> FileWatcherBuilder {
        FileWatcherBuilder::default()
    }

    pub(crate) fn from_builder(builder: FileWatcherBuilder) -> Self {
        let (file_tx, _) = tokio::sync::broadcast::channel(32);
        let file_tx_ = file_tx.clone();

        let mut debouncer = notify_debouncer_mini::new_debouncer(
            std::time::Duration::from_secs(builder.debounce_seconds),
            None,
            move |events: Result<
                Vec<notify_debouncer_mini::DebouncedEvent>,
                Vec<notify::Error>,
            >| {
                let e = events.unwrap().into_iter().map(|e| e.path).collect();
                file_tx_.send(e).unwrap();
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
        let (command_tx, command_rx) = tokio::sync::mpsc::channel(32);
        Self {
            file_tx,
            command_tx,
            command_rx,
            debouncer,
        }
    }
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for FileWatcher {
    type Client = FileWatcherClient;

    fn name<'a>() -> &'a str {
        "file_watcher_service"
    }

    async fn run(mut self, context: ServiceContext) {
        while let Ok(Some(command)) = self
            .command_rx
            .recv()
            .cancel_on_shutdown(&context.get_subsystem_handle())
            .await
        {
            match command {
                FileWatcherCommand::Watch(path, recursive_mode) => self
                    .debouncer
                    .watcher()
                    .watch(&path, recursive_mode)
                    .unwrap(),
                FileWatcherCommand::Unwatch(path) => {
                    self.debouncer.watcher().unwatch(&path).unwrap()
                }
            }
        }
        self.debouncer.stop();
    }

    async fn get_client(&mut self) -> Self::Client {
        FileWatcherClient::new(self.command_tx.clone())
    }
}

impl daemon_slayer_core::server::EventService for FileWatcher {
    type EventStoreImpl = BroadcastEventStore<Vec<PathBuf>>;

    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.file_tx.clone())
    }
}
