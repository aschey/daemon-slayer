use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

pub(crate) fn define_service(ident: Ident, crate_name: proc_macro2::TokenStream) -> TokenStream {
    let direct_handler = crate::windows_macros::get_direct_handler(&crate_name, &ident);
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

fn get_service_impl(
    crate_name: &proc_macro2::TokenStream,
    ident: &Ident,
    direct_handler: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        #[#crate_name::async_trait::async_trait]
        impl #crate_name::Service for #ident {
            async fn run_service_main() ->  Result<(), Box<dyn std::error::Error + Send + Sync>> {
                #crate_name::windows_service::service_dispatcher::start(#ident::get_service_name(), func_service_main)?;
                Ok(())
            }

            #direct_handler
        }
    }
}

fn get_service_main(
    crate_name: &proc_macro2::TokenStream,
    ident: &Ident,
) -> proc_macro2::TokenStream {
    quote! {
        #crate_name::platform::get_service_main::<#ident>();
    }
}

fn get_direct_handler(
    crate_name: &proc_macro2::TokenStream,
    ident: &Ident,
) -> proc_macro2::TokenStream {
    quote! {
        async fn run_service_direct() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            #crate_name::platform::get_direct_handler::<#ident>().await
        }
    }
}
