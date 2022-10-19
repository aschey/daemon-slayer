use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use daemon_slayer_core::server::blocking::BroadcastEventStore;
use once_cell::sync::OnceCell;

use crate::Signal;

pub struct SignalHandler {
    signal_tx: Arc<Mutex<bus::Bus<Signal>>>,
    shutdown_tx: std::sync::mpsc::Sender<()>,
    handle: JoinHandle<()>,
}

impl SignalHandler {
    pub fn set_sender(sender: Arc<Mutex<bus::Bus<Signal>>>) {
        SENDER.set(sender).unwrap();
    }
}

pub struct SignalBuilder {}

pub struct SignalClient {}

static SENDER: OnceCell<Arc<Mutex<bus::Bus<Signal>>>> = OnceCell::new();

impl daemon_slayer_core::server::blocking::Service for SignalHandler {
    type Builder = SignalBuilder;

    type Client = SignalClient;

    fn run_service(_: Self::Builder) -> Self {
        let signal_tx = SENDER
            .get()
            .map(|tx| tx.to_owned())
            .unwrap_or_else(|| Arc::new(Mutex::new(bus::Bus::new(32))));
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();
        let signal_tx_ = signal_tx.clone();
        let handle = thread::spawn(move || {
            let sig = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            signal_hook::flag::register(signal_hook::consts::SIGINT, std::sync::Arc::clone(&sig))
                .unwrap();

            loop {
                if sig.swap(false, Ordering::Relaxed) {
                    signal_tx_.lock().unwrap().broadcast(Signal::SIGINT);
                }
                if shutdown_rx.try_recv().is_ok() {
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        Self {
            shutdown_tx,
            signal_tx,
            handle,
        }
    }

    fn get_client(&mut self) -> Self::Client {
        SignalClient {}
    }

    fn stop(self) {
        self.shutdown_tx.send(()).unwrap();
        self.handle.join().unwrap();
    }
}

impl daemon_slayer_core::server::blocking::EventService for SignalHandler {
    type EventStoreImpl = BroadcastEventStore<Signal>;
    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}
