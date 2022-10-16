use daemon_slayer_core::{BroadcastEventStore, EventStore};

use signal_hook_tokio::SignalsInfo;

use crate::Signal;

pub struct SignalHandler {
    signal_tx: tokio::sync::broadcast::Sender<Signal>,
    signals_handle: signal_hook_tokio::Handle,
    handle: tokio::task::JoinHandle<()>,
}

pub struct SignalBuilder {
    signals: SignalsInfo,
}

impl Default for SignalBuilder {
    fn default() -> Self {
        let signals = signal_hook_tokio::Signals::new(&[]).unwrap();
        Self { signals }
    }
}

impl SignalBuilder {
    pub fn all() -> Self {
        let signals = signal_hook_tokio::Signals::new(&[
            signal_hook::consts::signal::SIGHUP,
            signal_hook::consts::signal::SIGTERM,
            signal_hook::consts::signal::SIGINT,
            signal_hook::consts::signal::SIGQUIT,
            signal_hook::consts::signal::SIGTSTP,
            signal_hook::consts::signal::SIGCHLD,
            signal_hook::consts::signal::SIGCONT,
        ])
        .unwrap();
        Self { signals }
    }
}

pub struct SignalClient {
    signals_handle: signal_hook_tokio::Handle,
}

impl SignalClient {
    pub fn add_signal(&self, signal: Signal) {
        let signal_int = match signal {
            Signal::SIGTERM => signal_hook::consts::signal::SIGTERM,
            Signal::SIGQUIT => signal_hook::consts::signal::SIGQUIT,
            Signal::SIGINT => signal_hook::consts::signal::SIGINT,
            Signal::SIGTSTP => signal_hook::consts::signal::SIGTSTP,
            Signal::SIGHUP => signal_hook::consts::signal::SIGHUP,
            Signal::SIGCHLD => signal_hook::consts::signal::SIGCHLD,
            Signal::SIGCONT => signal_hook::consts::signal::SIGCONT,
            Signal::Other(_) => return,
        };
        self.signals_handle.add_signal(signal_int).unwrap();
    }
}

#[async_trait::async_trait]
impl daemon_slayer_core::Service for SignalHandler {
    type Builder = SignalBuilder;

    type Client = SignalClient;

    async fn run_service(builder: Self::Builder) -> Self {
        let signals = builder.signals;
        let signals_handle = signals.handle();

        let (signal_tx, _) = tokio::sync::broadcast::channel(32);
        let signal_tx_ = signal_tx.clone();

        let handle = tokio::spawn(async move {
            use futures::stream::StreamExt;
            let mut signals = signals.fuse();
            while let Some(signal) = signals.next().await {
                let signal_name = signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
                signal_tx_.send(signal_name.into()).unwrap();
            }
        });
        Self {
            signal_tx,
            signals_handle,
            handle,
        }
    }

    fn get_client(&mut self) -> Self::Client {
        SignalClient {
            signals_handle: self.signals_handle.clone(),
        }
    }

    async fn stop(self) {
        self.signals_handle.close();
        self.handle.await.unwrap();
    }
}

impl daemon_slayer_core::EventService for SignalHandler {
    type EventStoreImpl = BroadcastEventStore<Signal>;
    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}
