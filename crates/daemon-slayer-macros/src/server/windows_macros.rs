use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

pub(crate) fn define_service(ident: Ident, crate_name: proc_macro2::TokenStream) -> TokenStream {
    quote! {
        static __INPUT_DATA: std::sync::OnceLock<
        Box<dyn #crate_name::AsAny + Send + Sync + 'static>> =
            std::sync::OnceLock::new();

        static __FINISHED_TX: std::sync::OnceLock<
        #crate_name::tokio::sync::mpsc::Sender<std::result::Result<(),
        #crate_name::ServiceError<<#ident as #crate_name::Handler>::Error>>>> =
            std::sync::OnceLock::new();

        #crate_name::windows_service::define_windows_service!(func_service_main,
            handle_service_main);

        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
            let boxed_data = __INPUT_DATA.get().expect("__INPUT_DATA should be set").as_any();
            let input_data = boxed_data.downcast_ref::<Option<<#ident as
            #crate_name::Handler>::InputData>>()
                .expect("__INPUT_DATA should be of type InputData")
                .clone();

            let res = #crate_name::platform::get_service_main::<#ident>(input_data);
            __FINISHED_TX
                .get()
                .expect("__FINISHED_TX not set")
                .try_send(res)
                .expect("Failed to send service result");
        }

        impl #crate_name::Service for #ident {
            async fn run_as_service(input_data: Option<Self::InputData>) -> std::result::Result<(),
            #crate_name::ServiceError<Self::Error>> {
                if let Err(e) = __INPUT_DATA.set(Box::new(input_data)) {
                    panic!("__INPUT_DATA already set");
                }

                let (tx, mut rx) = tokio::sync::mpsc::channel(1);
                __FINISHED_TX.set(tx).expect("__FINISHED_TX already set");

                #crate_name::windows_service::service_dispatcher::start(#ident::label().application,
                func_service_main)
                    .map_err(|e| #crate_name::ServiceError::InitializationFailure(
                        "Failed to start service dispatcher".to_owned(), Box::new(e)))?;

                // Wait for service main function to complete and get the result
                rx.recv().await.expect("Failed to receive service result")
            }

            async fn run_directly(input_data: Option<Self::InputData>) -> Result<(),
            #crate_name::ServiceError<Self::Error>> {
                #crate_name::platform::get_direct_handler::<#ident>(input_data).await
            }
        }
    }
    .into()
}
