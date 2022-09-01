use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

#[maybe_async::async_impl]
pub(crate) fn define_service(ident: Ident, crate_name: proc_macro2::TokenStream) -> TokenStream {
    let direct_handler = get_direct_handler();
    quote! {
        #[#crate_name::maybe_async::async_impl]
        impl #crate_name::service_manager::Service for #ident {
            async fn run_service_main() -> u32 {
                use #crate_name::service_manager::{ServiceHandler, StopHandler};
                let mut handler = #ident::new();
                let stop_handler = handler.get_stop_handler();

                let signals = #crate_name::signal_hook_tokio::Signals::new(&[
                    #crate_name::signal_hook::consts::signal::SIGHUP,
                    #crate_name::signal_hook::consts::signal::SIGTERM,
                    #crate_name::signal_hook::consts::signal::SIGINT,
                    #crate_name::signal_hook::consts::signal::SIGQUIT,
                ])
                .unwrap();

                let signals_handle = signals.handle();

                let signals_task = #crate_name::tokio::spawn(async move {
                    use #crate_name::futures::stream::StreamExt;

                    let mut signals = signals.fuse();
                    while let Some(signal) = signals.next().await {
                        match signal {
                            #crate_name::signal_hook::consts::signal::SIGTERM
                            | #crate_name::signal_hook::consts::signal::SIGINT
                            | #crate_name::signal_hook::consts::signal::SIGQUIT
                            | #crate_name::signal_hook::consts::signal::SIGHUP => {
                                #[cfg(target_os = "linux")]
                                #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Stopping]).unwrap();
                                stop_handler().await;
                            }
                            _ => {}
                        }
                    }
                });

                let status = handler.run_service(|| {
                    #[cfg(target_os = "linux")]
                    #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Ready]).unwrap();
                }).await;
                signals_handle.close();
                signals_task.await.unwrap();
                status
            }

            #direct_handler
        }
    }
    .into()
}

#[maybe_async::sync_impl]
pub(crate) fn define_service(ident: Ident, crate_name: proc_macro2::TokenStream) -> TokenStream {
    let direct_handler = get_direct_handler();
    quote! {
        #[#crate_name::maybe_async::sync_impl]
        impl #crate_name::service_manager::Service for #ident {
            fn run_service_main() -> u32 {
                use daemon_slayer::service_manager::{ServiceHandler, StopHandler};
                let mut handler = #ident::new();
                let stop_handler = handler.get_stop_handler();

                std::thread::spawn(move || {
                    let mut signals = #crate_name::signal_hook::iterator::Signals::new(&[
                        #crate_name::signal_hook::consts::signal::SIGHUP,
                        #crate_name::signal_hook::consts::signal::SIGTERM,
                        #crate_name::signal_hook::consts::signal::SIGINT,
                        #crate_name::signal_hook::consts::signal::SIGQUIT,
                    ])
                    .unwrap();

                    for signal in &mut signals {
                        match signal {
                            #crate_name::signal_hook::consts::signal::SIGTERM
                            | #crate_name::signal_hook::consts::signal::SIGINT
                            | #crate_name::signal_hook::consts::signal::SIGQUIT
                            | #crate_name::signal_hook::consts::signal::SIGHUP => {
                                #[cfg(target_os = "linux")]
                                #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Stopping]).unwrap();
                                stop_handler();
                            }
                            _ => {}
                        }
                    }
                });
                handler.run_service(|| {
                    #[cfg(target_os = "linux")]
                    #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Ready]).unwrap();
                })
            }

            #direct_handler
        }

    }
    .into()
}

#[cfg(not(feature = "direct"))]
fn get_direct_handler() -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

#[maybe_async::sync_impl]
#[cfg(feature = "direct")]
fn get_direct_handler() -> proc_macro2::TokenStream {
    quote! {
        fn run_service_direct(mut self) -> u32 {
            Self::run_service_main()
        }
    }
}

#[maybe_async::async_impl]
#[cfg(feature = "direct")]
fn get_direct_handler() -> proc_macro2::TokenStream {
    quote! {
        async fn run_service_direct(mut self) -> u32 {
            Self::run_service_main().await
        }
    }
}
