use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use daemon_slayer_core::blocking::BroadcastEventStore;

use crate::Signal;

use super::{
    signal_handler_builder::SignalHandlerBuilder, signal_handler_client::SignalHandlerClient,
};

pub struct SignalHandler {
    signal_tx: Arc<Mutex<bus::Bus<Signal>>>,
    signals_handle: signal_hook_tokio::Handle,
    handle: JoinHandle<()>,
}

impl daemon_slayer_core::blocking::Service for SignalHandler {
    type Builder = SignalHandlerBuilder;

    type Client = SignalHandlerClient;

    fn run_service(builder: Self::Builder) -> Self {
        let mut signals = builder.signals;
        let signals_handle = signals.handle();

        let signal_tx = Arc::new(Mutex::new(bus::Bus::new(32)));
        let signal_tx_ = signal_tx.clone();

        let handle = thread::spawn(move || {
            for signal in signals.forever() {
                let signal_name = signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
                signal_tx_.lock().unwrap().broadcast(signal_name.into());
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

    fn stop(self) {
        self.signals_handle.close();
        self.handle.join().unwrap();
    }
}

impl daemon_slayer_core::blocking::EventService for SignalHandler {
    type EventStoreImpl = BroadcastEventStore<Signal>;
    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}
