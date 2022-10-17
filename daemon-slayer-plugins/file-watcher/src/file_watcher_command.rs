use std::path::PathBuf;

use notify::RecursiveMode;

#[derive(Clone, Debug)]
pub(crate) enum FileWatcherCommand {
    Stop,
    Watch(PathBuf, RecursiveMode),
    Unwatch(PathBuf),
}
