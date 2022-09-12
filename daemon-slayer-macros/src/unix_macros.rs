use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

#[cfg(feature = "async")]
pub(crate) fn define_service_async(
    ident: Ident,
    crate_name: proc_macro2::TokenStream,
) -> TokenStream {
    let direct_handler = get_direct_handler_async();
    let service_main = get_service_main_async(&ident, &crate_name);

    quote! {
        #[#crate_name::async_trait::async_trait]
        impl #crate_name::ServiceAsync for #ident {
            #service_main

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
    let service_main = get_service_main_sync(&ident, &crate_name);

    quote! {
        impl #crate_name::ServiceSync for #ident {
            #service_main

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

#[cfg(all(feature = "signal-handler", feature = "async"))]
fn get_service_main_async(
    ident: &Ident,
    crate_name: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        async fn run_service_main(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            use #crate_name::HandlerAsync;
            let mut handler = #ident::new();
            let event_handler = handler.get_event_handler();

            let signals = #crate_name::signal_hook_tokio::Signals::new(&[
                #crate_name::signal_hook::consts::signal::SIGHUP,
                #crate_name::signal_hook::consts::signal::SIGTERM,
                #crate_name::signal_hook::consts::signal::SIGINT,
                #crate_name::signal_hook::consts::signal::SIGQUIT,
                #crate_name::signal_hook::consts::signal::SIGTSTP,
                #crate_name::signal_hook::consts::signal::SIGCHLD,
                #crate_name::signal_hook::consts::signal::SIGCONT,
            ])
            .unwrap();

            let signals_handle = signals.handle();

            let signals_task: #crate_name::tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> = #crate_name::tokio::spawn(async move {
                use #crate_name::futures::stream::StreamExt;

                let mut signals = signals.fuse();
                while let Some(signal) = signals.next().await {
                    #[cfg(target_os = "linux")]
                    #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Stopping]).unwrap();
                    let signal_name = #crate_name::signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
                    event_handler(#crate_name::Event::SignalReceived(signal_name.into())).await?;
                }
                Ok(())
            });

            let result = handler.run_service(|| {
                #[cfg(target_os = "linux")]
                #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Ready]).unwrap();
            }).await;

            signals_handle.close();
            signals_task.await.unwrap()?;
            result
        }
    }
}

#[cfg(all(not(feature = "signal-handler"), feature = "async"))]
fn get_service_main_async(
    ident: &Ident,
    crate_name: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        async fn run_service_main(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            use #crate_name::HandlerAsync;
            let mut handler = #ident::new();

            let result = handler.run_service(|| {
                #[cfg(target_os = "linux")]
                #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Ready]).unwrap();
            }).await;

            result
        }
    }
}

#[cfg(all(feature = "signal-handler", feature = "blocking"))]
fn get_service_main_sync(
    ident: &Ident,
    crate_name: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        fn run_service_main(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            use #crate_name::{HandlerSync, EventHandlerSync};
            let mut handler = #ident::new();
            let event_handler = handler.get_event_handler();

            let mut signals = #crate_name::signal_hook::iterator::Signals::new(&[
                #crate_name::signal_hook::consts::signal::SIGHUP,
                #crate_name::signal_hook::consts::signal::SIGTERM,
                #crate_name::signal_hook::consts::signal::SIGINT,
                #crate_name::signal_hook::consts::signal::SIGQUIT,
                #crate_name::signal_hook::consts::signal::SIGTSTP,
                #crate_name::signal_hook::consts::signal::SIGCHLD,
                #crate_name::signal_hook::consts::signal::SIGCONT,
            ])
            .unwrap();

            let signals_handle = signals.handle();

            let handle: std::thread::JoinHandle::<Result<(), Box<dyn std::error::Error + Send + Sync>>> = std::thread::spawn(move || {
                for signal in &mut signals {
                    #[cfg(target_os = "linux")]
                    #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Stopping]).unwrap();
                    let signal_name = #crate_name::signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
                    event_handler(#crate_name::Event::SignalReceived(signal_name.into()))?;
                }

                Ok(())
            });
            handler.run_service(|| {
                #[cfg(target_os = "linux")]
                #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Ready]).unwrap();
            })?;
            signals_handle.close();
            handle.join().unwrap()?;

            Ok(())
        }
    }
}

#[cfg(all(not(feature = "signal-handler"), feature = "blocking"))]
fn get_service_main_sync(
    ident: &Ident,
    crate_name: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        fn run_service_main(self: Box<Self>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            use #crate_name::HandlerSync;
            let mut handler = #ident::new();

            let result = handler.run_service(|| {
                #[cfg(target_os = "linux")]
                #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Ready]).unwrap();
            });

            result
        }
    }
}
