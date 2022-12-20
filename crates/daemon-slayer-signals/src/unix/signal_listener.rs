use daemon_slayer_core::{
    server::{BroadcastEventStore, ServiceContext},
    signal::{self, Signal},
    BoxedError,
};
use futures::stream::StreamExt;
use signal_hook_tokio::SignalsInfo;
use std::ffi::c_int;

use super::SignalListenerClient;

pub struct SignalListener {
    signals: SignalsInfo,
    signal_tx: tokio::sync::broadcast::Sender<Signal>,
}

impl Default for SignalListener {
    fn default() -> Self {
        let default_signals: [c_int; 0] = [];
        let (signal_tx, _) = tokio::sync::broadcast::channel(32);
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
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for SignalListener {
    type Client = SignalListenerClient;

    fn name<'a>() -> &'a str {
        "signal_listener_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let signals_handle = self.signals.handle();

        let mut signals = self.signals.fuse();
        while let Some(signal) = signals.next().await {
            let signal_name = signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
            let signal: Signal = signal_name.into();
            self.signal_tx.send(signal.clone()).ok();
            if let Signal::SIGTERM | Signal::SIGQUIT | Signal::SIGINT = signal {
                context.get_subsystem_handle().request_global_shutdown();
                signals_handle.close();
                return Ok(());
            }
        }
        Ok(())
    }

    async fn get_client(&mut self) -> Self::Client {
        SignalListenerClient::new(self.signals.handle())
    }
}

impl daemon_slayer_core::server::EventService for SignalListener {
    type EventStoreImpl = BroadcastEventStore<Signal>;
    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}
