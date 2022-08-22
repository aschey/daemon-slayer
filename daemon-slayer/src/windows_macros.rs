#[macro_export]
macro_rules! __internal_utils {
    (@define_service $service_func_name: ident) => {
        $crate::paste::paste! {
            $crate::windows_service::define_windows_service!([<func_ $service_func_name>], handle_service_main);
        }
    };
    (@start_service $handler_type: ident, $handler: ident, $on_stop: expr, $status_handle: ident) => {
        let stop_handler = $handler.get_stop_handler();
        let event_handler = move |control_event| -> $crate::windows_service::service_control_handler::ServiceControlHandlerResult {
            match control_event {
                // Notifies a service to report its current status information to the service
                // control manager. Always return NoError even if not implemented.
                $crate::windows_service::service::ServiceControl::Interrogate => $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError,

                // Handle stop
                $crate::windows_service::service::ServiceControl::Stop => {
                    $on_stop;
                    //$crate::futures::executor::block_on(async { stop_handler().await });
                    $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError
                }

                _ =>  $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NotImplemented,
            }
        };

        let $status_handle = match $crate::windows_service::service_control_handler::register($handler_type::get_service_name(), event_handler) {
            Ok(handle) => handle,
            Err(e) => {
                return;
            }
        };

        $status_handle
            .set_service_status($crate::windows_service::service::ServiceStatus {
                service_type: $crate::windows_service::service::ServiceType::OWN_PROCESS,
                current_state: $crate::windows_service::service::ServiceState::Running,
                controls_accepted: $crate::windows_service::service::ServiceControlAccept::STOP,
                exit_code: $crate::windows_service::service::ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: std::time::Duration::default(),
                process_id: None,
            })
            .unwrap();
    };

    (@stop_service $status_handle: ident, $exit_code: ident) => {
        $status_handle
            .set_service_status($crate::windows_service::service::ServiceStatus {
                service_type: $crate::windows_service::service::ServiceType::OWN_PROCESS,
                current_state: $crate::windows_service::service::ServiceState::Stopped,
                controls_accepted: $crate::windows_service::service::ServiceControlAccept::empty(),
                exit_code: $crate::windows_service::service::ServiceExitCode::Win32($exit_code),
                checkpoint: 0,
                wait_hint: std::time::Duration::default(),
                process_id: None,
            })
            .unwrap();
    };
}

#[cfg(all(feature = "async-tokio", feature = "direct"))]
#[macro_export]
macro_rules! __internal_direct_handler {
    ($func_name: ident, $service_handler: ident) => {
        pub async fn $func_name() -> u32 {
            let mut handler = $service_handler::new();
            let stop_handler = handler.get_stop_handler();
            $crate::tokio::spawn(async move {
                $crate::tokio::signal::ctrl_c().await.unwrap();
                stop_handler().await;
            });

            handler.run_service().await
        }
    };
}

#[cfg(all(not(feature = "async-tokio"), feature = "direct"))]
#[macro_export]
macro_rules! __internal_direct_handler {
    ($func_name: ident, $service_handler: ident) => {
        pub fn $func_name() -> u32 {
            let mut handler = $service_handler::new();
            let stop_handler = handler.get_stop_handler();
            std::thread::spawn(move || {
                let term = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                $crate::signal_hook::flag::register(
                    $crate::signal_hook::consts::SIGTERM,
                    std::sync::Arc::clone(&term),
                )
                .unwrap();
                $crate::signal_hook::flag::register(
                    $crate::signal_hook::consts::SIGINT,
                    std::sync::Arc::clone(&term),
                )
                .unwrap();
                while !term.load(std::sync::atomic::Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                stop_handler();
            });

            handler.run_service()
        }
    };
}

#[cfg(not(feature = "direct"))]
#[macro_export]
macro_rules! __internal_direct_handler {
    ($func_name: ident, $service_handler: ident) => {};
}

#[cfg(feature = "async-tokio")]
#[macro_export]
macro_rules! define_service {
    ($service_func_name:ident, $service_handler:ident) => {
        $crate::__internal_utils!(@define_service $service_func_name);

        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
            let mut handler = $service_handler::new();
            let stop_handler = handler.get_stop_handler();
            $crate::__internal_utils!(@start_service $service_handler, handler, $crate::futures::executor::block_on(async { stop_handler().await }), status_handle);
            let rt = $crate::tokio::runtime::Runtime::new().unwrap();
            let exit_code = rt.block_on(async { handler.run_service().await });

            $crate::__internal_utils!(@stop_service status_handle, exit_code);
        }

        $crate::paste::paste! {
            pub async fn $service_func_name() {
                $crate::windows_service::service_dispatcher::start($service_handler::get_service_name(), [<func_ $service_func_name>]).unwrap();
            }

            pub async fn [<$service_func_name _main>]() -> u32 {
                let mut handler = $service_handler::new();
                let stop_handler = handler.get_stop_handler();
                $crate::tokio::spawn(async move {
                    $crate::tokio::signal::ctrl_c().await.unwrap();
                    stop_handler().await;
                });

                handler.run_service().await
            }
        }
    };
}

#[cfg(not(feature = "async-tokio"))]
#[macro_export]
macro_rules! define_service {
    ($service_func_name:ident, $service_handler:ident) => {
        $crate::__internal_utils!(@define_service $service_func_name);
        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
            let mut handler = $service_handler::new();
            let stop_handler = handler.get_stop_handler();
            $crate::__internal_utils!(@start_service $service_handler, handler, stop_handler(), status_handle);
            let exit_code = handler.run_service();
            $crate::__internal_utils!(@stop_service status_handle, exit_code);
        }

        $crate::paste::paste! {
            pub fn $service_func_name() {
                $crate::windows_service::service_dispatcher::start($service_handler::get_service_name(), [<func_ $service_func_name>]).unwrap();
            }

            $crate::__internal_direct_handler!([<$service_func_name _main>], $service_handler);
        }
    };
}
