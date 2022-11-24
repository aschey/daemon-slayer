use crate::{Signal, SignalHandlerTrait};
use daemon_slayer_core::server::{BroadcastEventStore, SubsystemHandle};
use futures::stream::StreamExt;
use signal_hook_tokio::SignalsInfo;
use std::ffi::c_int;

use super::signal_handler_client::SignalHandlerClient;

pub struct SignalHandler {
    signals: SignalsInfo,
    signal_tx: tokio::sync::broadcast::Sender<Signal>,
}

impl Default for SignalHandler {
    fn default() -> Self {
        let default_signals: [c_int; 0] = [];
        let (signal_tx, _) = tokio::sync::broadcast::channel(32);
        Self {
            signal_tx,
            signals: signal_hook_tokio::Signals::new(default_signals).unwrap(),
        }
    }
}

impl SignalHandlerTrait for SignalHandler {
    fn all() -> Self {
        let signals = signal_hook_tokio::Signals::new([
            signal_hook::consts::signal::SIGHUP,
            signal_hook::consts::signal::SIGTERM,
            signal_hook::consts::signal::SIGINT,
            signal_hook::consts::signal::SIGQUIT,
            signal_hook::consts::signal::SIGTSTP,
            signal_hook::consts::signal::SIGCHLD,
            signal_hook::consts::signal::SIGCONT,
        ])
        .unwrap();
        Self {
            signals,
            ..Self::default()
        }
    }
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for SignalHandler {
    type Client = SignalHandlerClient;

    async fn run(mut self, subsys: SubsystemHandle) {
        let signals_handle = self.signals.handle();

        let (signal_tx, _) = tokio::sync::broadcast::channel(32);
        let signal_tx_ = signal_tx.clone();

        let mut signals = self.signals.fuse();
        while let Some(signal) = signals.next().await {
            let signal_name = signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
            let signal: Signal = signal_name.into();
            signal_tx_.send(signal.clone()).ok();
            if let Signal::SIGTERM | Signal::SIGQUIT | Signal::SIGINT = signal {
                subsys.request_global_shutdown();
                signals_handle.close();
            }
        }
    }

    async fn get_client(&mut self) -> Self::Client {
        SignalHandlerClient::new(self.signals.handle())
    }
}

impl daemon_slayer_core::server::EventService for SignalHandler {
    type EventStoreImpl = BroadcastEventStore<Signal>;
    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}
