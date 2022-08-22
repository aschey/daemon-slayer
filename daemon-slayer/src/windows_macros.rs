#[macro_export]
macro_rules! __internal_utils {
    (@start_service $sender: ident, $service_name: ident, $define_handler:expr, $on_stop:expr, $status_handle: ident) => {
         // Create a channel to be able to poll a stop event from the service worker loop.
        let sender_ = $sender.clone();
        let event_handler = move |control_event| -> $crate::windows_service::service_control_handler::ServiceControlHandlerResult {
            match control_event {
                // Notifies a service to report its current status information to the service
                // control manager. Always return NoError even if not implemented.
                $crate::windows_service::service::ServiceControl::Interrogate => $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError,

                // Handle stop
                $crate::windows_service::service::ServiceControl::Stop => {
                    $on_stop(&sender_);
                    $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError
                }

                _ =>  $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NotImplemented,
            }
        };

        let $status_handle = match $crate::windows_service::service_control_handler::register($service_name, event_handler) {
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

    (@service_main $service_name:ident, $service_func_name:ident) => {
        $crate::paste::paste! {
            pub async fn $service_func_name() {
                $crate::windows_service::service_dispatcher::start($service_name, [<func_ $service_func_name>]).unwrap();
            }
        }
    };

    (@direct_func_body $sender: ident, $on_stop: expr) => {
        let sender_ = $sender.clone();
        $crate::ctrlc::set_handler(move || {
            $on_stop(&sender_);
        })
        .unwrap();
    };
}

#[cfg(feature = "async-tokio")]
#[macro_export]
macro_rules! define_service {
    ($service_name:ident, $service_func_name:ident, $define_handler:expr, $on_stop:expr, $service_main_func:ident) => {
        $crate::paste::paste! {
            $crate::windows_service::define_windows_service!([<func_ $service_func_name>], handle_service_main);
        }

        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
            let (sender, receiver) = $define_handler;
             // Create a channel to be able to poll a stop event from the service worker loop.
            let sender_ = sender.clone();

            let event_handler = move |control_event| -> $crate::windows_service::service_control_handler::ServiceControlHandlerResult {
                match control_event {
                    // Notifies a service to report its current status information to the service
                    // control manager. Always return NoError even if not implemented.
                    $crate::windows_service::service::ServiceControl::Interrogate => $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError,

                    // Handle stop
                    $crate::windows_service::service::ServiceControl::Stop => {
                        $crate::futures::executor::block_on(async { $on_stop(sender_.clone()).await });
                        $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError
                    }

                    _ =>  $crate::windows_service::service_control_handler::ServiceControlHandlerResult::NotImplemented,
                }
            };

            let status_handle = match $crate::windows_service::service_control_handler::register($service_name, event_handler) {
                Ok(handle) => handle,
                Err(e) => {
                    return;
                }
            };

            status_handle
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

                let rt = $crate::tokio::runtime::Runtime::new().unwrap();
                let exit_code = rt.block_on(async { $service_main_func(sender, receiver).await });

                $crate::__internal_utils!(@stop_service status_handle, exit_code);
            }

        $crate::__internal_utils!(@service_main $service_name, $service_func_name);

        $crate::paste::paste! {
            pub async fn [<$service_func_name _main>]() -> u32 {
                let (sender, receiver) = $define_handler;
                let sender_ = sender.clone();

                $crate::ctrlc::set_handler(move || {
                    $crate::futures::executor::block_on(async { $on_stop(sender_.clone()).await });
                })
                .unwrap();
                $service_main_func(sender, receiver).await
            }
        }
    };
}

#[cfg(not(feature = "async-tokio"))]
#[macro_export]
macro_rules! define_service {
    ($service_name:ident, $service_func_name:ident, $define_handler:expr, $on_stop:expr, $service_main_func:ident) => {
        $crate::paste::paste! {
            $crate::windows_service::define_windows_service!([<func_ $service_func_name>], handle_service_main);
        }

        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
            let (sender, receiver) = $define_handler;
            $crate::__internal_utils!(@start_service sender, $service_name, $define_handler, $on_stop, status_handle);

            let rt = $crate::tokio::runtime::Runtime::new().unwrap();
            let exit_code = $service_main_func(sender, receiver);

            $crate::__internal_utils!(@stop_service status_handle, exit_code);
        }

        $crate::__internal_utils!(@service_main $service_name, $service_func_name);

        $crate::paste::paste! {
            pub fn [<$service_func_name _main>]() -> u32 {
                let (sender, receiver) = $define_handler;
                $crate::__internal_direct_func_body!(sender, $on_stop);
                $service_main_func(sender, receiver)
            }
        }
    };
}
