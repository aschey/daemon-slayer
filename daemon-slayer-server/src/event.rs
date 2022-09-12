use std::path::PathBuf;

use crate::Signal;

#[derive(Debug, Clone)]
pub enum Event {
    SignalReceived(Signal),
    FileChanged(Vec<PathBuf>),
}
