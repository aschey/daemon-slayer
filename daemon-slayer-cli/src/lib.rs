#[cfg(any(feature = "client", feature = "server"))]
mod action;
#[cfg(any(feature = "client", feature = "server"))]
pub use action::*;

#[cfg(any(feature = "client", feature = "server"))]
mod command;
#[cfg(any(feature = "client", feature = "server"))]
mod commands;
#[cfg(any(feature = "client", feature = "server"))]
mod input_state;
#[cfg(any(feature = "client", feature = "server"))]
pub use input_state::InputState;
#[cfg(any(feature = "client", feature = "server"))]
mod service_command;
#[cfg(any(feature = "client", feature = "server"))]
pub use service_command::*;

#[cfg(any(feature = "client", feature = "server"))]
pub use command::*;
#[cfg(any(feature = "client", feature = "server"))]
mod builder;
#[cfg(any(feature = "client", feature = "server"))]
pub use builder::*;
#[cfg(any(feature = "client", feature = "server"))]
mod cli;
pub use clap;
#[cfg(any(feature = "client", feature = "server"))]
pub use cli::*;

mod cli2;
