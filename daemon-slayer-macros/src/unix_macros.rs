use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

#[cfg(feature = "async")]
pub(crate) fn define_service_async(
    ident: Ident,
    crate_name: proc_macro2::TokenStream,
) -> TokenStream {
    let direct_handler = get_direct_handler_async();
    quote! {
        #[#crate_name::async_trait::async_trait]
        impl #crate_name::ServiceAsync for #ident {
            async fn run_service_main(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                #crate_name::platform::run_service_main_async::<#ident>().await
            }

            #direct_handler
        }
    }
    .into()
}

#[cfg(feature = "blocking")]
pub(crate) fn define_service_sync(
    ident: Ident,
    crate_name: proc_macro2::TokenStream,
) -> TokenStream {
    let direct_handler = get_direct_handler_sync();
    quote! {
        impl #crate_name::ServiceSync for #ident {
            fn run_service_main(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                #crate_name::platform::run_service_main_sync::<#ident>()
            }

            #direct_handler
        }

    }
    .into()
}

#[cfg(not(feature = "direct"))]
fn get_direct_handler_async() -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

#[cfg(not(feature = "direct"))]
fn get_direct_handler_sync() -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

#[cfg(all(feature = "direct", feature = "blocking"))]
fn get_direct_handler_sync() -> proc_macro2::TokenStream {
    quote! {
        fn run_service_direct(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.run_service_main()
        }
    }
}

#[cfg(all(feature = "direct", feature = "async"))]
fn get_direct_handler_async() -> proc_macro2::TokenStream {
    quote! {
        async fn run_service_direct(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.run_service_main().await
        }
    }
}
