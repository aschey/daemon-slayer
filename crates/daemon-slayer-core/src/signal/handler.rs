use std::sync::OnceLock;

use tap::TapFallible;
use tokio::sync::broadcast;
use tracing::error;

use super::Signal;

static SENDER: OnceLock<broadcast::Sender<Signal>> = OnceLock::new();

pub fn set_sender(tx: broadcast::Sender<Signal>) {
    SENDER
        .set(tx)
        .tap_err(|e| error!("Error setting signal sender: {e:#?}"))
        .ok();
}

pub fn get_sender() -> Option<broadcast::Sender<Signal>> {
    SENDER.get().map(|tx| tx.to_owned())
}

pub trait Handler: Default {
    fn all() -> Self;
    fn termination() -> Self;
}
