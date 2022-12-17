use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

#[cfg(windows)]
mod windows_macros;

#[cfg(unix)]
mod unix_macros;

#[cfg(windows)]
pub mod platform {
    pub(crate) use super::windows_macros::*;
}

#[cfg(unix)]
pub mod platform {
    pub(crate) use super::unix_macros::*;
}
