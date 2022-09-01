#[cfg(not(feature = "direct"))]
#[macro_export]
macro_rules! __internal_direct_handler {
    ($func_name: ident, $service_handler: ident) => {};
}

#[cfg(all(not(feature = "async-tokio"), feature = "direct"))]
#[macro_export]
macro_rules! __internal_direct_handler {
    ($func_name: ident, $service_handler: ident) => {
        $crate::paste::paste! {
            pub fn [<$func_name _main>]() -> u32 {
                $func_name()
            }
        }
    };
}

#[cfg(all(feature = "async-tokio", feature = "direct"))]
#[macro_export]
macro_rules! __internal_direct_handler {
    ($func_name: ident, $service_handler: ident) => {
        $crate::paste::paste! {
            pub async fn [<$func_name _main>]() -> u32  {
                $func_name().await
            }
        }
    };
}

#[cfg(feature = "async-tokio")]
#[macro_export]
macro_rules! define_service {
    ($service_func_name:ident, $service_handler:ident) => {
        pub async fn $service_func_name() -> u32 {
            use daemon_slayer::service_manager::{ServiceHandler, StopHandler};
            let mut handler = $service_handler::new();
            let stop_handler = handler.get_stop_handler();

            let signals = $crate::signal_hook_tokio::Signals::new(&[
                $crate::signal_hook::consts::signal::SIGHUP,
                $crate::signal_hook::consts::signal::SIGTERM,
                $crate::signal_hook::consts::signal::SIGINT,
                $crate::signal_hook::consts::signal::SIGQUIT,
            ])
            .unwrap();

            let signals_handle = signals.handle();

            let signals_task = $crate::tokio::spawn(async move {
                use $crate::futures::stream::StreamExt;

                let mut signals = signals.fuse();
                while let Some(signal) = signals.next().await {
                    match signal {
                        $crate::signal_hook::consts::signal::SIGTERM
                        | $crate::signal_hook::consts::signal::SIGINT
                        | $crate::signal_hook::consts::signal::SIGQUIT
                        | $crate::signal_hook::consts::signal::SIGHUP => {
                            #[cfg(target_os = "linux")]
                            sd_notify::notify(false, &[sd_notify::NotifyState::Stopping]).unwrap();
                            stop_handler().await;
                        }
                        _ => {}
                    }
                }
            });

            let status = handler
                .run_service(|| {
                    #[cfg(target_os = "linux")]
                    sd_notify::notify(false, &[sd_notify::NotifyState::Ready]).unwrap();
                })
                .await;
            signals_handle.close();
            signals_task.await.unwrap();

            status
        }

        $crate::__internal_direct_handler!($service_func_name, $service_handler);
    };
}

#[cfg(not(feature = "async-tokio"))]
#[macro_export]
macro_rules! define_service {
    ($service_func_name:ident, $service_handler:ident) => {
        pub fn $service_func_name() -> u32 {
            use daemon_slayer::service_manager::{ServiceHandler, StopHandler};
            let mut handler = $service_handler::new();
            let stop_handler = handler.get_stop_handler();

            std::thread::spawn(move || {
                let mut signals = $crate::signal_hook::iterator::Signals::new(&[
                    $crate::signal_hook::consts::signal::SIGHUP,
                    $crate::signal_hook::consts::signal::SIGTERM,
                    $crate::signal_hook::consts::signal::SIGINT,
                    $crate::signal_hook::consts::signal::SIGQUIT,
                ])
                .unwrap();

                for signal in &mut signals {
                    match signal {
                        $crate::signal_hook::consts::signal::SIGTERM
                        | $crate::signal_hook::consts::signal::SIGINT
                        | $crate::signal_hook::consts::signal::SIGQUIT
                        | $crate::signal_hook::consts::signal::SIGHUP => {
                            #[cfg(target_os = "linux")]
                            sd_notify::notify(false, &[sd_notify::NotifyState::Stopping]).unwrap();
                            stop_handler();
                        }
                        _ => {}
                    }
                }
            });

            handler.run_service(|| {
                #[cfg(target_os = "linux")]
                sd_notify::notify(false, &[sd_notify::NotifyState::Ready]).unwrap();
            })
        }

        $crate::__internal_direct_handler!($service_func_name, $service_handler);
    };
}
