use daemon_slayer_core::{
    server::{BroadcastEventStore, ServiceContext, SubsystemHandle},
    signal::{self, Signal},
};

use super::SignalListenerClient;

pub struct SignalListener {
    signal_tx: tokio::sync::broadcast::Sender<Signal>,
}

impl Default for SignalListener {
    fn default() -> Self {
        let signal_tx = signal::get_sender().unwrap_or_else(|| {
            let (tx, _) = tokio::sync::broadcast::channel(32);
            tx
        });

        Self { signal_tx }
    }
}

impl signal::Handler for SignalListener {
    fn all() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for SignalListener {
    type Client = SignalListenerClient;

    async fn run(self, context: ServiceContext) {
        let subsys = context.get_subsystem_handle();
        let mut ctrl_c_stream = tokio::signal::windows::ctrl_c().unwrap();
        let mut ctrl_break_stream = tokio::signal::windows::ctrl_break().unwrap();
        let mut ctrl_shutdown_stream = tokio::signal::windows::ctrl_shutdown().unwrap();
        let mut ctrl_logoff_stream = tokio::signal::windows::ctrl_logoff().unwrap();
        let mut ctrl_close_stream = tokio::signal::windows::ctrl_close().unwrap();

        loop {
            tokio::select! {
                _ = ctrl_c_stream.recv() => { self.signal_tx.send(Signal::SIGINT).ok() }
                _ = ctrl_break_stream.recv() => {  self.signal_tx.send(Signal::SIGINT).ok() }
                _ = ctrl_shutdown_stream.recv() => {  self.signal_tx.send(Signal::SIGINT).ok() }
                _ = ctrl_logoff_stream.recv() => {  self.signal_tx.send(Signal::SIGINT).ok() }
                _ = ctrl_close_stream.recv() => {  self.signal_tx.send(Signal::SIGINT).ok() }
                _ = subsys.on_shutdown_requested() => { return; }
            };
            subsys.request_global_shutdown();
        }
    }

    async fn get_client(&mut self) -> Self::Client {
        SignalListenerClient {}
    }
}

impl daemon_slayer_core::server::EventService for SignalListener {
    type EventStoreImpl = BroadcastEventStore<Signal>;
    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}
