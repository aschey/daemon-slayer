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
    pub(crate) use crate::windows_macros::*;
}

#[cfg(unix)]
mod platform {
    pub(crate) use crate::unix_macros::*;
}

#[cfg(feature = "async")]
#[proc_macro_derive(ServiceAsync)]
pub fn derive_service_async(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let found_crate = crate_name("daemon-slayer-server").unwrap();

    let crate_name = match found_crate {
        FoundCrate::Itself => quote!(daemon_slayer_server),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
    };

    platform::define_service_async(ident, crate_name)
}

#[cfg(feature = "blocking")]
#[proc_macro_derive(ServiceSync)]
pub fn derive_service_sync(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let found_crate = crate_name("daemon-slayer-server").unwrap();

    let crate_name = match found_crate {
        FoundCrate::Itself => quote!(daemon_slayer_server),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
    };

    platform::define_service_sync(ident, crate_name)
}
