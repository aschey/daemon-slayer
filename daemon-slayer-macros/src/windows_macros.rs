use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

#[maybe_async_cfg::maybe(
    idents(
        Handler,
        EventHandler,
        get_service_main(snake),
        get_direct_handler(snake),
        get_service_impl(snake)
    ),
    sync(feature = "blocking"),
    async(feature = "async")
)]
pub(crate) fn define_service(ident: Ident, crate_name: proc_macro2::TokenStream) -> TokenStream {
    let direct_handler = crate::windows_macros::get_direct_handler(&crate_name);
    let service_main = crate::windows_macros::get_service_main(&crate_name, &ident);

    let service_impl =
        crate::windows_macros::get_service_impl(&crate_name, &ident, &direct_handler);

    quote! {
        #crate_name::windows_service::define_windows_service!(func_service_main, handle_service_main);

        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
            #service_main
        }

        #service_impl
    }
    .into()
}

#[cfg(feature = "async")]
fn get_service_impl_async(
    crate_name: &proc_macro2::TokenStream,
    ident: &Ident,
    direct_handler: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        #[#crate_name::async_trait::async_trait]
        impl #crate_name::ServiceAsync for #ident {
            async fn run_service_main(self: Box<Self>) ->  Result<(), Box<dyn std::error::Error + Send + Sync>> {
                #crate_name::windows_service::service_dispatcher::start(#ident::get_service_name(), func_service_main)?;
                Ok(())
            }

            #direct_handler
        }
    }
}

#[cfg(feature = "blocking")]
fn get_service_impl_sync(
    crate_name: &proc_macro2::TokenStream,
    ident: &Ident,
    direct_handler: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        impl #crate_name::ServiceSync for #ident {
            fn run_service_main(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                #crate_name::windows_service::service_dispatcher::start(#ident::get_service_name(), func_service_main)?;
                Ok(())
            }

            #direct_handler
        }
    }
}

#[cfg(feature = "async")]
fn get_service_main_async(
    crate_name: &proc_macro2::TokenStream,
    ident: &Ident,
) -> proc_macro2::TokenStream {
    quote! {
        #crate_name::platform::get_service_main_async::<#ident>();
    }
}

#[cfg(feature = "blocking")]
fn get_service_main_sync(
    crate_name: &proc_macro2::TokenStream,
    ident: &Ident,
) -> proc_macro2::TokenStream {
    quote! {
        #crate_name::platform::get_service_main_sync::<#ident>();
    }
}

#[cfg(not(feature = "direct"))]
fn get_direct_handler_sync(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

#[cfg(not(feature = "direct"))]
fn get_direct_handler_async(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

#[cfg(all(feature = "direct", feature = "async"))]
fn get_direct_handler_async(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        async fn run_service_direct(mut self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            #crate_name::platform::get_direct_handler_async(*self).await
        }
    }
}

#[cfg(all(feature = "direct", feature = "blocking"))]
fn get_direct_handler_sync(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        fn run_service_direct(mut self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            #crate_name::platform::get_direct_handler_sync(*self)
        }
    }
}
