use crate::Signal;

#[derive(Debug, Clone)]
pub enum Event {
    SignalReceived(Signal),
}
