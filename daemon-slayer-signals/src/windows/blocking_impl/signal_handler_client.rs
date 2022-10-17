use crate::{signal_handler_client::SignalHandlerClientTrait, Signal};

pub struct SignalHandlerClient {}

impl SignalHandlerClientTrait for SignalHandlerClient {
    fn add_signal(&self, _: Signal) {}
}
