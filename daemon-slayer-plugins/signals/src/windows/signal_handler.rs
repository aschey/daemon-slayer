use crate::Signal;
use daemon_slayer_core::server::BroadcastEventStore;
use once_cell::sync::OnceCell;

use super::{SignalHandlerBuilder, SignalHandlerClient};

pub struct SignalHandler {
    signal_tx: tokio::sync::broadcast::Sender<Signal>,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

static SENDER: OnceCell<tokio::sync::broadcast::Sender<Signal>> = OnceCell::new();

impl SignalHandler {
    pub fn set_sender(tx: tokio::sync::broadcast::Sender<Signal>) {
        SENDER.set(tx).unwrap();
    }
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::Service for SignalHandler {
    type Builder = SignalHandlerBuilder;
    type Client = SignalHandlerClient;

    async fn run_service(_: Self::Builder) -> Self {
        let signal_tx = SENDER.get().map(|tx| tx.to_owned()).unwrap_or_else(|| {
            let (tx, _) = tokio::sync::broadcast::channel(32);
            tx
        });
        let signal_tx_ = signal_tx.clone();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(32);

        let handle = tokio::spawn(async move {
            let mut ctrl_c_stream = tokio::signal::windows::ctrl_c().unwrap();
            let mut ctrl_break_stream = tokio::signal::windows::ctrl_break().unwrap();
            let mut ctrl_shutdown_stream = tokio::signal::windows::ctrl_shutdown().unwrap();
            let mut ctrl_logoff_stream = tokio::signal::windows::ctrl_logoff().unwrap();
            let mut ctrl_close_stream = tokio::signal::windows::ctrl_close().unwrap();

            loop {
                tokio::select! {
                    _ = ctrl_c_stream.recv() => { signal_tx_.send(Signal::SIGINT).ok() }
                    _ = ctrl_break_stream.recv() => { signal_tx_.send(Signal::SIGINT).ok() }
                    _ = ctrl_shutdown_stream.recv() => { signal_tx_.send(Signal::SIGINT).ok() }
                    _ = ctrl_logoff_stream.recv() => { signal_tx_.send(Signal::SIGINT).ok() }
                    _ = ctrl_close_stream.recv() => { signal_tx_.send(Signal::SIGINT).ok() }
                    _ = shutdown_rx.recv() => { return; }
                };
            }
        });

        Self {
            shutdown_tx,
            signal_tx,
            handle,
        }
    }

    fn get_client(&mut self) -> Self::Client {
        SignalHandlerClient {}
    }

    async fn stop(self) {
        self.shutdown_tx.send(()).await.ok();
        self.handle.await.unwrap();
    }
}

impl daemon_slayer_core::server::EventService for SignalHandler {
    type EventStoreImpl = BroadcastEventStore<Signal>;
    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}
