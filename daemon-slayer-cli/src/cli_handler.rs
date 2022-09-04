use std::error::Error;

use crate::action::Action;

#[maybe_async::maybe_async]
pub trait CliHandler {
    async fn handle_input(self) -> Result<bool, Box<dyn Error>>;
    fn action_type(&self) -> Action;
}
