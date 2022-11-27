use daemon_slayer_core::signal::{self, Signal};

pub struct SignalListenerClient {
    signals_handle: signal_hook_tokio::Handle,
}

impl SignalListenerClient {
    pub(crate) fn new(signals_handle: signal_hook_tokio::Handle) -> Self {
        Self { signals_handle }
    }
}

impl signal::Client for SignalHandlerClient {
    fn add_signal(&self, signal: Signal) {
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
