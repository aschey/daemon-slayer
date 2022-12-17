use daemon_slayer_core::signal::{self, Signal};

pub struct SignalListenerClient {}

impl signal::Client for SignalListenerClient {
    fn add_signal(&self, _: Signal) {}
}
