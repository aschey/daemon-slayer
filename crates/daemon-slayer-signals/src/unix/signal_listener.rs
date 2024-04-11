use std::ffi::c_int;

use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::BroadcastEventStore;
use daemon_slayer_core::signal::{self, Signal};
use daemon_slayer_core::{BoxedError, FutureExt};
use futures::stream::StreamExt;
use signal_hook_tokio::SignalsInfo;
use tokio::sync::broadcast;

use super::SignalListenerClient;

pub struct SignalListener {
    signals: SignalsInfo,
    signal_tx: broadcast::Sender<Signal>,
}

impl SignalListener {
    pub fn get_client(&self) -> SignalListenerClient {
        SignalListenerClient::new(self.signals.handle())
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<Signal> {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}

impl Default for SignalListener {
    fn default() -> Self {
        let default_signals: [c_int; 0] = [];
        let (signal_tx, _) = broadcast::channel(32);
        Self {
            signal_tx,
            signals: signal_hook_tokio::Signals::new(default_signals).unwrap(),
        }
    }
}

impl signal::Handler for SignalListener {
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

    fn termination() -> Self {
        let signals = signal_hook_tokio::Signals::new([
            signal_hook::consts::signal::SIGTERM,
            signal_hook::consts::signal::SIGINT,
            signal_hook::consts::signal::SIGQUIT,
            signal_hook::consts::signal::SIGTSTP,
        ])
        .unwrap();
        Self {
            signals,
            ..Self::default()
        }
    }
}

impl BackgroundService for SignalListener {
    fn name(&self) -> &str {
        "signal_listener_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let signals_handle = self.signals.handle();
        let cancellation_token = context.cancellation_token();
        let mut signals = self.signals.fuse();
        while let Ok(Some(signal)) = signals.next().cancel_on_shutdown(&cancellation_token).await {
            let signal_name = signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
            let signal: Signal = signal_name.into();
            self.signal_tx.send(signal.clone()).ok();
            if let Signal::SIGTERM | Signal::SIGQUIT | Signal::SIGINT = signal {
                context.cancellation_token().cancel();
                signals_handle.close();
                return Ok(());
            }
        }
        Ok(())
    }
}
