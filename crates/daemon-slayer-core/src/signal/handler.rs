use once_cell::sync::OnceCell;
use tap::TapFallible;
use tokio::sync::broadcast;
use tracing::error;

use super::Signal;

static SENDER: OnceCell<broadcast::Sender<Signal>> = OnceCell::new();

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
}
