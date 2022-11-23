use crate::{Signal, SignalHandlerClientTrait};

pub struct SignalHandlerClient {}

impl SignalHandlerClientTrait for SignalHandlerClient {
    fn add_signal(&self, _: Signal) {}
}
