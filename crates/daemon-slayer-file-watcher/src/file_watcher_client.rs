use std::path::PathBuf;

use notify::RecursiveMode;
use tokio::sync::mpsc;

use crate::file_watcher_command::FileWatcherCommand;

#[derive(Clone)]
pub struct FileWatcherClient {
    command_tx: mpsc::Sender<FileWatcherCommand>,
}

impl FileWatcherClient {
    pub(crate) fn new(command_tx: mpsc::Sender<FileWatcherCommand>) -> Self {
        Self { command_tx }
    }

    pub async fn watch_path(&self, path: PathBuf, recursive_mode: RecursiveMode) {
        self.command_tx
            .send(FileWatcherCommand::Watch(path, recursive_mode))
            .await;
    }

    pub async fn unwatch_path(&self, path: PathBuf) {
        self.command_tx
            .send(FileWatcherCommand::Unwatch(path))
            .await;
    }
}
