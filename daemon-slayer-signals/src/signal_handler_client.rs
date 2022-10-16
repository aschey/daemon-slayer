use crate::Signal;

pub trait SignalHandlerClientTrait {
    fn add_signal(&self, signal: Signal);
}
