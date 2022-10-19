mod signal;
pub use signal::*;

mod signal_handler_builder;
pub use signal_handler_builder::*;

mod signal_handler_client;
pub use signal_handler_client::*;

#[cfg(unix)]
mod unix;
#[cfg(all(unix, feature = "async-tokio"))]
pub use unix::async_handler::*;
#[cfg(all(unix, feature = "blocking"))]
pub mod blocking {
    pub use crate::unix::blocking_handler::*;
}

#[cfg(windows)]
mod windows;
#[cfg(all(windows, feature = "async-tokio"))]
pub use windows::async_handler::*;
#[cfg(all(windows, feature = "blocking"))]
pub mod blocking {
    pub use crate::windows::blocking_handler::*;
}