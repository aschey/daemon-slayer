use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::Debouncer;

use crate::file_watcher_command::FileWatcherCommand;

#[derive(Clone)]
pub struct FileWatcherClient {
    command_tx: std::sync::mpsc::Sender<FileWatcherCommand>,
}

impl FileWatcherClient {
    pub(crate) fn new(command_tx: std::sync::mpsc::Sender<FileWatcherCommand>) -> Self {
        Self { command_tx }
    }

    pub fn watch_path(&self, path: PathBuf, recursive_mode: RecursiveMode) {
        self.command_tx
            .send(FileWatcherCommand::Watch(path, recursive_mode));
    }

    pub fn unwatch_path(&self, path: PathBuf) {
        self.command_tx.send(FileWatcherCommand::Unwatch(path));
    }
}
