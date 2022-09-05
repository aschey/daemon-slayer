use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::Ident;

#[maybe_async_cfg::maybe(
    idents(Handler, StopHandler, get_stop_fn(snake), get_direct_handler(snake), run_service_main(snake), get_imports(snake), get_service_impl(snake)),
    sync(feature = "blocking"),
    async(feature = "async")
)]
pub(crate) fn define_service(ident: Ident, crate_name: proc_macro2::TokenStream) -> TokenStream {
    let stop_fn = crate::windows_macros::get_stop_fn(&crate_name);
    let direct_handler = crate::windows_macros::get_direct_handler(&crate_name);
    let service_main = crate::windows_macros::run_service_main(&crate_name);
    let imports = crate::windows_macros::get_imports(&crate_name);
    let service_impl = crate::windows_macros::get_service_impl(&crate_name, &ident, &direct_handler);

    quote! {
        #crate_name::windows_service::define_windows_service!(func_service_main, handle_service_main);
        
        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
            #imports

            let mut handler = #ident::new();
            let stop_handler = handler.get_stop_handler();

            let event_handler = move |control_event| -> #crate_name::windows_service::service_control_handler::ServiceControlHandlerResult {
                match control_event {
                    // Notifies a service to report its current status information to the service
                    // control manager. Always return NoError even if not implemented.
                    #crate_name::windows_service::service::ServiceControl::Interrogate => #crate_name::windows_service::service_control_handler::ServiceControlHandlerResult::NoError,

                    
                    // Handle stop
                    #crate_name::windows_service::service::ServiceControl::Stop => {
                        #stop_fn
                        #crate_name::windows_service::service_control_handler::ServiceControlHandlerResult::NoError
                    }

                    _ => #crate_name::windows_service::service_control_handler::ServiceControlHandlerResult::NotImplemented,
                }
            };

            let status_handle = match #crate_name::windows_service::service_control_handler::register(#ident::get_service_name(), event_handler) {
                Ok(handle) => std::sync::Arc::new(std::sync::Mutex::new(handle)),
                Err(e) => {
                    return;
                }
            };
            let status_handle_ = status_handle.clone();
            let on_started = move || {
                status_handle_.lock().unwrap()
                    .set_service_status(#crate_name::windows_service::service::ServiceStatus {
                        service_type: #crate_name::windows_service::service::ServiceType::OWN_PROCESS,
                        current_state: #crate_name::windows_service::service::ServiceState::Running,
                        controls_accepted: #crate_name::windows_service::service::ServiceControlAccept::STOP,
                        exit_code: #crate_name::windows_service::service::ServiceExitCode::Win32(0),
                        checkpoint: 0,
                        wait_hint: std::time::Duration::default(),
                        process_id: None,
                    })
                    .unwrap();
            };

            #service_main;

            status_handle.lock().unwrap()
                .set_service_status(#crate_name::windows_service::service::ServiceStatus {
                    service_type: #crate_name::windows_service::service::ServiceType::OWN_PROCESS,
                    current_state: #crate_name::windows_service::service::ServiceState::Stopped,
                    controls_accepted: #crate_name::windows_service::service::ServiceControlAccept::empty(),
                    exit_code: #crate_name::windows_service::service::ServiceExitCode::Win32(exit_code),
                    checkpoint: 0,
                    wait_hint: std::time::Duration::default(),
                    process_id: None,
                })
                .unwrap();
        }

        #service_impl

    }
    .into()
}

#[cfg(feature="async")]
fn get_service_impl_async(crate_name: &proc_macro2::TokenStream, ident: &Ident, direct_handler: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        #[#crate_name::async_trait::async_trait]
        impl #crate_name::ServiceAsync for #ident {
            async fn run_service_main() -> u32 {
                #crate_name::windows_service::service_dispatcher::start(#ident::get_service_name(), func_service_main).unwrap();
                0
            }

            #direct_handler
        }
    }
}

#[cfg(feature="blocking")]
fn get_service_impl_sync(crate_name: &proc_macro2::TokenStream, ident: &Ident, direct_handler: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        impl #crate_name::ServiceSync for #ident {
            fn run_service_main() -> u32 {
                #crate_name::windows_service::service_dispatcher::start(#ident::get_service_name(), func_service_main).unwrap();
                0
            }

            #direct_handler
        }
    }
}

#[cfg(feature="async")]
fn get_imports_async(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        use #crate_name::{HandlerAsync, StopHandlerAsync};
    }
}

#[cfg(feature="blocking")]
fn get_imports_sync(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        use #crate_name::{HandlerSync, StopHandlerSync};
    }
}

#[cfg(feature="async")]
fn get_stop_fn_async(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        #crate_name::futures::executor::block_on(async { stop_handler().await });
    }
}

#[cfg(feature="blocking")]
fn get_stop_fn_sync(_: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        stop_handler();
    }
}

#[cfg(feature="async")]
fn run_service_main_async(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        let rt = #crate_name::tokio::runtime::Runtime::new().unwrap();
        let exit_code = rt.block_on(async { handler.run_service(on_started).await });
    }
}

#[cfg(feature="blocking")]
fn run_service_main_sync(_: &proc_macro2::TokenStream)-> proc_macro2::TokenStream {
    quote! {
        let exit_code = handler.run_service(on_started);
    }
}

#[cfg(not(feature="direct"))]
fn get_direct_handler_sync(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

#[cfg(not(feature="direct"))]
fn get_direct_handler_async(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::new()
}

#[cfg(all(feature="direct", feature="async"))]
fn get_direct_handler_async(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        async fn run_service_direct(mut self) -> u32 {
            let stop_handler = self.get_stop_handler();
            #crate_name::tokio::spawn(async move {
                #crate_name::tokio::signal::ctrl_c().await.unwrap();
                stop_handler().await;
            });

            self.run_service(|| {}).await
        }
    }
}

#[cfg(all(feature="direct", feature="blocking"))]
fn get_direct_handler_sync(crate_name: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        fn run_service_direct(mut self) -> u32 {
            let stop_handler = self.get_stop_handler();
            std::thread::spawn(move || {
                let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                #crate_name::signal_hook::flag::register(
                    #crate_name::signal_hook::consts::SIGTERM,
                    std::sync::Arc::clone(&term),
                )
                .unwrap();
                #crate_name::signal_hook::flag::register(
                    #crate_name::signal_hook::consts::SIGINT,
                    std::sync::Arc::clone(&term),
                )
                .unwrap();
                while !term.load(std::sync::atomic::Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                stop_handler();
            });

            self.run_service(|| {})           
        }
    }
}