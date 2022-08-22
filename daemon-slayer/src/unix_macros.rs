#[cfg(feature = "async-tokio")]
#[macro_export]
macro_rules! define_service {
    ($service_name:ident, $service_func_name:ident, $define_handler:expr, $on_stop:expr, $service_main_func:ident) => {
        $crate::paste::paste! {
            pub async fn [<$service_func_name _main>]() -> u32 {
                let (sender, receiver) = $define_handler;
                let sender_ = sender.clone();

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
                                $on_stop(&sender_);
                            }
                            _ => {}
                        }
                    }
                });

                let status = $service_main_func(sender, receiver).await;
                signals_handle.close();
                signals_task.await.unwrap();
                status
            }

            pub async fn $service_func_name()  {
                [<$service_func_name _main>]().await;
            }
        }
    };
}

#[cfg(not(feature = "async-tokio"))]
#[macro_export]
macro_rules! define_service {
    ($service_name:ident, $service_func_name:ident, $define_handler:expr, $on_stop:expr, $service_main_func:ident) => {
        $crate::paste::paste! {
            pub fn [<$service_func_name _main>]() -> u32 {
                let (sender, receiver) = $define_handler;
                let sender_ = sender.clone();

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
                                $on_stop(&sender_);
                            }
                            _ => {}
                        }
                    }
                });

                $service_main_func(sender, receiver)
            }

            pub fn $service_func_name()  {
                [<$service_func_name _main>]();
            }
        }
    };
}
