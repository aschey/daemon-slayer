use std::error::Error;

use crate::{action::Action, input_state::InputState};

#[maybe_async_cfg::maybe(
    idents(CliHandler),
    sync(feature = "blocking"),
    async(feature = "async-tokio", "async_trait::async_trait(?Send)")
)]
pub trait CliHandler {
    async fn handle_input(self) -> Result<InputState, Box<dyn Error>>;
    fn action_type(&self) -> Action;
}
