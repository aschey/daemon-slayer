mod signal;
pub use signal::*;

#[cfg(unix)]
mod unix;
#[cfg(all(unix, feature = "async-tokio"))]
pub use unix::async_impl::*;
#[cfg(all(unix, feature = "blocking"))]
pub mod blocking {
    pub use crate::unix::blocking_impl::*;
}
