use std::{
    error::Error,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use daemon_slayer_core::server::blocking::BroadcastEventStore;
use notify::RecommendedWatcher;
use notify_debouncer_mini::Debouncer;
use tracing::{error, info};

use crate::{file_watcher_builder::FileWatcherBuilder, file_watcher_command::FileWatcherCommand};

use super::file_watcher_client::FileWatcherClient;

pub struct FileWatcher {
    file_tx: Arc<Mutex<bus::Bus<Vec<PathBuf>>>>,
    command_tx: std::sync::mpsc::Sender<FileWatcherCommand>,
    handle: JoinHandle<()>,
}

impl daemon_slayer_core::server::blocking::Service for FileWatcher {
    type Builder = FileWatcherBuilder;

    type Client = FileWatcherClient;

    fn run_service(builder: Self::Builder) -> Self {
        let file_tx = Arc::new(Mutex::new(bus::Bus::new(32)));
        let file_tx_ = file_tx.clone();
        let mut debouncer = notify_debouncer_mini::new_debouncer(
            std::time::Duration::from_secs(builder.debounce_seconds),
            None,
            move |events: Result<
                Vec<notify_debouncer_mini::DebouncedEvent>,
                Vec<notify::Error>,
            >| {
                let e = events.unwrap().into_iter().map(|e| e.path).collect();
                file_tx_.lock().unwrap().broadcast(e);
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
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            while let Ok(command) = command_rx.recv() {
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

    fn stop(self) {
        self.command_tx.send(FileWatcherCommand::Stop).unwrap();
        self.handle.join().unwrap();
    }
}

impl daemon_slayer_core::server::blocking::EventService for FileWatcher {
    type EventStoreImpl = BroadcastEventStore<Vec<PathBuf>>;

    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.file_tx.clone())
    }
}
