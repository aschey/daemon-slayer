use std::{
    env::current_exe,
    ffi::{OsStr, OsString},
};

use windows_service::{
    service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType,
        ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

use crate::service_state::ServiceState;

#[macro_export]
macro_rules! define_service {
    ($service_name:ident, $service_func_name:ident, $define_handler:expr, $on_stop:expr, $service_main_func:ident) => {
        $crate::paste::paste! {
            $crate::windows_service::define_windows_service!([<func_ $service_func_name>], handle_service_main);
        }

        pub fn handle_service_main(args: Vec<std::ffi::OsString>) {
            // Create a channel to be able to poll a stop event from the service worker loop.
            let (sender, receiver) = $define_handler;
            let sender_ = sender.clone();
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

                let exit_code = $service_main_func(args, sender, receiver);

                status_handle.set_service_status($crate::windows_service::service::ServiceStatus {
                    service_type:  $crate::windows_service::service::ServiceType::OWN_PROCESS,
                    current_state: $crate::windows_service::service::ServiceState::Stopped,
                    controls_accepted: $crate::windows_service::service::ServiceControlAccept::empty(),
                    exit_code: $crate::windows_service::service::ServiceExitCode::Win32(exit_code),
                    checkpoint: 0,
                    wait_hint: std::time::Duration::default(),
                    process_id: None,
                }).unwrap();
        }

        $crate::paste::paste! {
            pub fn $service_func_name() {
                $crate::windows_service::service_dispatcher::start($service_name, [<func_ $service_func_name>]).unwrap();
            }

            pub fn [<$service_func_name _main>]() -> u32 {
                let (sender, receiver) = $define_handler;
                let sender_ = sender.clone();
                $crate::ctrlc::set_handler(move || {
                    $on_stop(&sender_);
                }).unwrap();

                $service_main_func(vec![], sender, receiver)
            }
        }
    };
}

pub struct Manager {
    service_manager: ServiceManager,
    service_name: String,
}
impl Manager {
    pub fn new<T: Into<String>>(service_name: T) -> Self {
        let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access).unwrap();
        Self {
            service_manager,
            service_name: service_name.into(),
        }
    }

    pub fn install<T: Into<String>>(&self, args: Vec<T>) {
        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
        if self
            .service_manager
            .open_service(&self.service_name, service_access)
            .is_err()
        {
            let service_binary_path = current_exe().unwrap();

            let service_info = ServiceInfo {
                name: OsString::from(&self.service_name),
                display_name: OsString::from(&self.service_name),
                service_type: ServiceType::OWN_PROCESS,
                start_type: ServiceStartType::OnDemand,
                error_control: ServiceErrorControl::Normal,
                executable_path: service_binary_path,
                launch_arguments: args.into_iter().map(|a| OsString::from(a.into())).collect(),
                dependencies: vec![],
                account_name: None, // run as System
                account_password: None,
            };
            let service = self
                .service_manager
                .create_service(
                    &service_info,
                    ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
                )
                .unwrap();
            service.set_description(&self.service_name).unwrap();
        }
    }

    pub fn uninstall(&self) {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::STOP
            | ServiceAccess::DELETE
            | ServiceAccess::START;
        let service = self
            .service_manager
            .open_service(&self.service_name, service_access)
            .unwrap();
        service.delete().unwrap();
    }

    pub fn start(&self) {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::STOP
            | ServiceAccess::DELETE
            | ServiceAccess::START;
        let service = self
            .service_manager
            .open_service(&self.service_name, service_access)
            .unwrap();
        service.start(&[OsStr::new("Started")]).unwrap();
    }

    pub fn stop(&self) {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::STOP
            | ServiceAccess::DELETE
            | ServiceAccess::START;
        let service = self
            .service_manager
            .open_service(&self.service_name, service_access)
            .unwrap();
        let _ = service.stop();
    }

    pub fn query_status(&self) -> ServiceState  {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::STOP
            | ServiceAccess::DELETE
            | ServiceAccess::START;
        let service = match self
            .service_manager
            .open_service(&self.service_name, service_access) {
                Ok(service) => service,
                Err(_) => return ServiceState::NotInstalled
            };
        match service.query_status().unwrap().current_state {
            windows_service::service::ServiceState::Stopped | windows_service::service::ServiceState::StartPending =>ServiceState::Stopped,
           _ => ServiceState::Started,
        }
    }

    pub fn is_installed(&self) -> bool {
    self.query_status() != ServiceState::NotInstalled
    }
}
