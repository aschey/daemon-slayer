use std::{
    error::Error,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use daemon_slayer_core::server::BroadcastEventStore;
use notify::RecommendedWatcher;
use notify_debouncer_mini::Debouncer;
use tracing::{error, info};

use crate::{file_watcher_builder::FileWatcherBuilder, file_watcher_command::FileWatcherCommand};

use super::file_watcher_client::FileWatcherClient;

pub struct FileWatcher {
    file_tx: tokio::sync::broadcast::Sender<Vec<PathBuf>>,
    command_tx: tokio::sync::mpsc::Sender<FileWatcherCommand>,
    handle: tokio::task::JoinHandle<()>,
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::Service for FileWatcher {
    type Builder = FileWatcherBuilder;

    type Client = FileWatcherClient;

    async fn run_service(builder: Self::Builder) -> Self {
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
                file_tx_.send(e);
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
        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(32);
        let handle = tokio::spawn(async move {
            while let Some(command) = command_rx.recv().await {
                match command {
                    FileWatcherCommand::Stop => {
                        debouncer.stop();
                        return;
                    }
                    FileWatcherCommand::Watch(path, recursive_mode) => {
                        debouncer.watcher().watch(&path, recursive_mode).unwrap()
                    }
                    FileWatcherCommand::Unwatch(path) => {
                        debouncer.watcher().unwatch(&path).unwrap()
                    }
                }
            }
        });

        Self {
            file_tx,
            command_tx,
            handle,
        }
    }

    fn get_client(&mut self) -> Self::Client {
        FileWatcherClient::new(self.command_tx.clone())
    }

    async fn stop(self) {
        self.command_tx
            .send(FileWatcherCommand::Stop)
            .await
            .unwrap();
        self.handle.await.unwrap();
    }
}

impl daemon_slayer_core::server::EventService for FileWatcher {
    type EventStoreImpl = BroadcastEventStore<Vec<PathBuf>>;

    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.file_tx.clone())
    }
}
