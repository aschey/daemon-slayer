use crate::SignalHandlerBuilderTrait;

#[derive(Default)]
pub struct SignalHandlerBuilder {}

impl SignalHandlerBuilderTrait for SignalHandlerBuilder {
    fn all() -> Self {
        Self {}
    }
}
