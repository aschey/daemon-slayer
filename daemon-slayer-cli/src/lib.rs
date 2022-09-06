#[cfg(any(feature = "client", feature = "server"))]
mod action;
#[cfg(any(feature = "client", feature = "server"))]
pub use action::Action;
#[cfg(any(feature = "client", feature = "server"))]
mod builder;
#[cfg(any(feature = "client", feature = "server"))]
mod command;
#[cfg(any(feature = "client", feature = "server"))]
mod commands;
#[cfg(any(feature = "client", feature = "server"))]
mod service_commands;

#[cfg(any(feature = "client", feature = "server"))]
mod cli_handler;
#[cfg(any(feature = "client", feature = "server"))]
pub use cli_handler::*;
#[cfg(any(feature = "client", feature = "server"))]
mod util;
#[cfg(any(feature = "client", feature = "server"))]
pub use command::Command;
#[cfg(feature = "client")]
mod client;
#[cfg(all(feature = "server", feature = "client"))]
mod combined;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "client")]
pub use client::*;
#[cfg(all(feature = "server", feature = "client"))]
pub use combined::*;
#[cfg(feature = "server")]
pub use server::*;