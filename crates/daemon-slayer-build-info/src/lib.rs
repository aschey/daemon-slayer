#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "cli")]
pub use console::Color;
#[cfg(feature = "vergen")]
pub use vergen;
#[cfg(feature = "cli")]
pub use vergen_pretty;
