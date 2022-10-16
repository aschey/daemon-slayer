use signal_hook_tokio::SignalsInfo;

use crate::signal_handler_builder::SignalHandlerBuilderTrait;

pub struct SignalHandlerBuilder {
    pub(crate) signals: SignalsInfo,
}

impl Default for SignalHandlerBuilder {
    fn default() -> Self {
        let signals = signal_hook_tokio::Signals::new(&[]).unwrap();
        Self { signals }
    }
}

impl SignalHandlerBuilderTrait for SignalHandlerBuilder {
    fn all() -> Self {
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
