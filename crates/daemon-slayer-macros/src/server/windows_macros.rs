use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

pub(crate) fn define_service(ident: Ident, crate_name: proc_macro2::TokenStream) -> TokenStream {
    quote! {
        static __INPUT_DATA: #crate_name::once_cell::sync::OnceCell<Box<dyn #crate_name::AsAny + Send + Sync + 'static>> = 
            #crate_name::once_cell::sync::OnceCell::new();

        #crate_name::windows_service::define_windows_service!(func_service_main, handle_service_main);


        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
            let boxed_data = __INPUT_DATA.get().unwrap().as_any();
            let input_data = boxed_data.downcast_ref::<Option<<#ident as #crate_name::Handler>::InputData>>().unwrap().clone();
            #crate_name::platform::get_service_main::<#ident>(input_data);
        }

        #[#crate_name::async_trait::async_trait]
        impl #crate_name::Service for #ident {
            async fn run_as_service(input_data: Option<Self::InputData>) ->  Result<(), #crate_name::ServiceError<Self::Error>> {
                if let Err(e) = __INPUT_DATA.set(Box::new(input_data)) {
                    panic!("set data failed");
                }
                #crate_name::windows_service::service_dispatcher::start(#ident::label().application, func_service_main)
                    .map_err(|e| #crate_name::ServiceError::InitializationFailure("Failed to start service dispatcher".to_owned(), Box::new(e)))?;
                Ok(())
            }

            async fn run_directly(input_data: Option<Self::InputData>) -> Result<(), #crate_name::ServiceError<Self::Error>> {
                #crate_name::platform::get_direct_handler::<#ident>(input_data).await
            }
        }
    }
    .into()
}
