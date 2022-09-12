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
                use #crate_name::{HandlerAsync, EventHandlerAsync};
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

                let (file_tx, mut file_rx) = #crate_name::tokio::sync::mpsc::channel(32);
                let debouncer = start_file_watcher(handler.get_watch_paths(), file_tx);

                let signals_handle = signals.handle();

                let event_task: #crate_name::tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> = #crate_name::tokio::spawn(async move {
                    use #crate_name::futures::stream::StreamExt;

                    let mut signals = signals.fuse();

                    loop {
                        #crate_name::tokio::select! {
                            files = file_rx.recv() => {
                                match files {
                                    Some(files) => {
                                        event_handler(#crate_name::Event::FileChanged(files)).await?;
                                    },
                                    None => {
                                        return Ok(());
                                    }
                                }
                            }
                            signal = signals.next() => {
                                match signal {
                                    Some(signal) => {
                                        #[cfg(target_os = "linux")]
                                        #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Stopping]).unwrap();
                                        let signal_name = #crate_name::signal_hook::low_level::signal_name(signal).unwrap_or("unknown");
                                        event_handler(#crate_name::Event::SignalReceived(signal_name.into())).await?;
                                    },
                                    None => {
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                });

                let result = handler.run_service(|| {
                    #[cfg(target_os = "linux")]
                    #crate_name::sd_notify::notify(false, &[#crate_name::sd_notify::NotifyState::Ready]).unwrap();
                }).await;

                signals_handle.close();
                drop(debouncer);
                event_task.await.unwrap()?;
                result
            }

            #direct_handler
        }

        fn start_file_watcher(paths: &[std::path::PathBuf], 
            tx: #crate_name::tokio::sync::mpsc::Sender<Vec<std::path::PathBuf>>) -> 
            #crate_name::notify_debouncer_mini::Debouncer<#crate_name::notify::RecommendedWatcher> {
            let (watch_tx, watch_rx) = std::sync::mpsc::channel();
            let mut debouncer =
                #crate_name::notify_debouncer_mini::new_debouncer(std::time::Duration::from_secs(2), None, watch_tx).unwrap();
            let watcher = debouncer.watcher();
        
            for path in paths {
                watcher
                    .watch(path, #crate_name::notify::RecursiveMode::Recursive)
                    .unwrap();
            }

            #crate_name::tokio::task::spawn_blocking(move || {
                for events in watch_rx {
                    let e = events.unwrap().into_iter().map(|e| e.path).collect();
                    tx.blocking_send(e).unwrap();
                }
            });

            debouncer
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
