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
mod platform {
    pub(crate) use crate::windows_macros::define_service;
}

#[cfg(unix)]
mod platform {
    pub(crate) use crate::unix_macros::define_service;
}

#[proc_macro_derive(Service)]
pub fn derive_service(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let found_crate = crate_name("daemon-slayer-server").unwrap();

    let crate_name = match found_crate {
        FoundCrate::Itself => quote!(daemon_slayer),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
    };

    platform::define_service(ident, crate_name)
}
