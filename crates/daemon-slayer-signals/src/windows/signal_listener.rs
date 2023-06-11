use super::SignalListenerClient;
use daemon_slayer_core::{
    async_trait,
    server::{BroadcastEventStore, ServiceContext},
    signal::{self, Signal},
    BoxedError,
};
use tap::TapFallible;
use tokio::sync::broadcast;
use tracing::{info, warn};

pub struct SignalListener {
    signal_tx: broadcast::Sender<Signal>,
}

impl SignalListener {
    pub fn get_client(&self) -> SignalListenerClient {
        SignalListenerClient {}
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<Signal> {
        BroadcastEventStore::new(self.signal_tx.clone())
    }
}

impl Default for SignalListener {
    fn default() -> Self {
        let signal_tx = signal::get_sender().unwrap_or_else(|| {
            let (tx, _) = broadcast::channel(32);
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

#[async_trait]
impl daemon_slayer_core::server::BackgroundService for SignalListener {
    fn name(&self) -> &str {
        "signal_listener_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let cancellation_token = context.cancellation_token();
        let mut signal_rx = self.signal_tx.subscribe();
        info!("Registering signal handlers");
        let mut ctrl_c_stream = tokio::signal::windows::ctrl_c().unwrap();
        let mut ctrl_break_stream = tokio::signal::windows::ctrl_break().unwrap();
        let mut ctrl_shutdown_stream = tokio::signal::windows::ctrl_shutdown().unwrap();
        let mut ctrl_logoff_stream = tokio::signal::windows::ctrl_logoff().unwrap();
        let mut ctrl_close_stream = tokio::signal::windows::ctrl_close().unwrap();

        tokio::select! {
            _ = ctrl_c_stream.recv() => {
                info!("Received ctrl+c signal");
                self.signal_tx.send(Signal::SIGINT).tap_err(|_| warn!("Failed to send signal")).ok();
            }
            _ = ctrl_break_stream.recv() => {
                self.signal_tx.send(Signal::SIGINT).tap_err(|_| warn!("Failed to send signal")).ok();
            }
            _ = ctrl_shutdown_stream.recv() => {
                self.signal_tx.send(Signal::SIGINT).tap_err(|_| warn!("Failed to send signal")).ok();
            }
            _ = ctrl_logoff_stream.recv() => {
                self.signal_tx.send(Signal::SIGINT).tap_err(|_| warn!("Failed to send signal")).ok();
            }
            _ = ctrl_close_stream.recv() => {
                self.signal_tx.send(Signal::SIGINT).tap_err(|_| warn!("Failed to send signal")).ok();
            }
            _ = signal_rx.recv() => {
                info!("Received signal from channel");
            }
            _ = cancellation_token.cancelled() => {
                info!("Shutdown requested. Stopping signal handler.");
                self.signal_tx.send(Signal::SIGINT).tap_err(|_| warn!("Failed to send signal")).ok();
                return Ok(());
            }
        };

        info!("Signal received. Requesting global shutdown.");
        cancellation_token.cancel();
        return Ok(());
    }
}
