use daemon_slayer_core::BroadcastEventStore;

use crate::Signal;

use super::{
    signal_handler_builder::SignalHandlerBuilder, signal_handler_client::SignalHandlerClient,
};

pub struct SignalHandler {
    signal_tx: tokio::sync::broadcast::Sender<Signal>,
    signals_handle: signal_hook_tokio::Handle,
    handle: tokio::task::JoinHandle<()>,
}

#[async_trait::async_trait]
impl daemon_slayer_core::Service for SignalHandler {
    type Builder = SignalHandlerBuilder;

    type Client = SignalHandlerClient;

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
        SignalHandlerClient::new(self.signals_handle.clone())
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
