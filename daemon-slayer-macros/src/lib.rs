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
#[proc_macro_derive(Service)]
pub fn derive_service_async(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let crate_name = get_crate_name();
    platform::define_service_async(ident, crate_name)
}

#[cfg(feature = "blocking")]
#[proc_macro_derive(BlockingService)]
pub fn derive_service_sync(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let crate_name = get_crate_name();
    platform::define_service_sync(ident, crate_name)
}

fn get_crate_name() -> proc_macro2::TokenStream {
    let server_crate = crate_name("daemon-slayer-server");
    let main_crate = crate_name("daemon-slayer");

    match (main_crate, server_crate) {
        (Ok(FoundCrate::Itself), _) => quote!(daemon_slayer::server),
        (Ok(FoundCrate::Name(name)), _) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident::server )
        }
        (_, Ok(FoundCrate::Itself)) => quote!(daemon_slayer_server),
        (_, Ok(FoundCrate::Name(name))) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
        _ => panic!("server crate not found"),
    }
}
