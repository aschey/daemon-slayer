use crate::Signal;

#[derive(Debug, Clone)]
pub enum Event {
    #[cfg(any(feature = "signal-handler-async", feature = "signal-handler-sync"))]
    SignalReceived(Signal),
}
