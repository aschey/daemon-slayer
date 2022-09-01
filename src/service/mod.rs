use std::{error::Error, result};

mod handler;
pub use handler::{Handler, StopHandler};
mod manager;
pub use manager::Manager;

mod platform;
pub use platform::ServiceManager;

mod status;
pub use status::Status;

mod builder;
pub use builder::Builder;

pub type Result<T> = result::Result<T, Box<dyn Error>>;

#[maybe_async::maybe_async]
pub trait Service {
    async fn run_service_main() -> u32;
    #[cfg(feature = "direct")]
    async fn run_service_direct(self) -> u32;
}
