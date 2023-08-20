mod server;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

#[cfg(feature = "server")]
#[proc_macro_derive(Service, attributes(DataType))]
pub fn derive_service(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let crate_name = get_crate_name(
        "daemon-slayer-server",
        quote!(daemon_slayer::server),
        quote!(daemon_slayer_server),
    );
    server::platform::define_service(ident, crate_name)
}

#[cfg(feature = "config")]
#[proc_macro_derive(Mergeable)]
pub fn derive_mergeable(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);

    let crate_name = get_crate_name(
        "daemon-slayer-core",
        quote!(daemon_slayer::core),
        quote!(daemon_slayer_core),
    );
    quote! {
        impl #crate_name::config::Mergeable for #ident {
            fn merge(user_config: Option<&Self>, app_config: &Self) -> Self {
                user_config.unwrap_or(app_config).to_owned()
            }
        }
    }
    .into()
}

fn get_crate_name(
    source_crate_name: &str,
    main_tokens: proc_macro2::TokenStream,
    source_tokens: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let source_crate = crate_name(source_crate_name);
    let main_crate = crate_name("daemon-slayer");

    match (main_crate, source_crate) {
        (Ok(FoundCrate::Itself), _) => main_tokens,
        (Ok(FoundCrate::Name(name)), _) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident::server )
        }
        (_, Ok(FoundCrate::Itself)) => source_tokens,
        (_, Ok(FoundCrate::Name(name))) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident )
        }
        _ => panic!("source crate not found"),
    }
}
