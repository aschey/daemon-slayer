use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

pub(crate) fn define_service(ident: Ident, crate_name: proc_macro2::TokenStream) -> TokenStream {
    quote! {
        #[#crate_name::async_trait]
        impl #crate_name::Service for #ident {
            async fn run_as_service(input_data: Option<Self::InputData>) -> Result<(), #crate_name::ServiceError<Self::Error>> {
                #crate_name::platform::run_as_service::<#ident>(input_data).await
            }

            async fn run_directly(input_data: Option<Self::InputData>) -> Result<(), #crate_name::ServiceError<Self::Error>> {
                Self::run_as_service(input_data).await
            }
        }
    }
    .into()
}
