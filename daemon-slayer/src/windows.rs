use windows_service::{
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager as WindowsServiceManager, ServiceManagerAccess},
};

use crate::{
    service_config::ServiceConfig, service_manager::ServiceManager, service_status::ServiceStatus,
};

#[macro_export]
macro_rules! define_service {
    ($service_name:ident, $service_func_name:ident, $define_handler:expr, $on_stop:expr, $service_main_func:ident) => {
        $crate::paste::paste! {
            $crate::windows_service::define_windows_service!([<func_ $service_func_name>], handle_service_main);
        }

        pub fn handle_service_main(_: Vec<std::ffi::OsString>) {
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

                let exit_code = $service_main_func(sender, receiver);

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

                $service_main_func(sender, receiver)
            }
        }
    };
}

pub struct Manager {
    service_manager: WindowsServiceManager,
    config: ServiceConfig,
}
impl ServiceManager for Manager {
    fn new(config: ServiceConfig) -> Self {
        let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
        let service_manager =
            WindowsServiceManager::local_computer(None::<&str>, manager_access).unwrap();
        Self {
            service_manager,
            config,
        }
    }

    fn install(&self) {
        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
        if self
            .service_manager
            .open_service(&self.config.name, service_access)
            .is_err()
        {
            let service_info = ServiceInfo {
                name: (&self.config.name).into(),
                display_name: (&self.config.display_name).into(),
                service_type: ServiceType::OWN_PROCESS,
                start_type: ServiceStartType::OnDemand,
                error_control: ServiceErrorControl::Normal,
                executable_path: (&self.config.program).into(),
                launch_arguments: self.config.args_iter().map(Into::into).collect(),
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
            service.set_description(&self.config.description).unwrap();
        }
    }

    fn uninstall(&self) {
        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
        let service = self
            .service_manager
            .open_service(&self.config.name, service_access)
            .unwrap();
        service.delete().unwrap();
    }

    fn start(&self) {
        let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::START;
        let service = self
            .service_manager
            .open_service(&self.config.name, service_access)
            .unwrap();
        service.start::<String>(&[]).unwrap();
    }

    fn stop(&self) {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::STOP
            | ServiceAccess::DELETE
            | ServiceAccess::START;
        let service = self
            .service_manager
            .open_service(&self.config.name, service_access)
            .unwrap();
        let _ = service.stop();
    }

    fn query_status(&self) -> ServiceStatus {
        let service_access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::STOP
            | ServiceAccess::DELETE
            | ServiceAccess::START;
        let service = match self
            .service_manager
            .open_service(&self.config.name, service_access)
        {
            Ok(service) => service,
            Err(_) => return ServiceStatus::NotInstalled,
        };
        match service.query_status().unwrap().current_state {
            windows_service::service::ServiceState::Stopped
            | windows_service::service::ServiceState::StartPending => ServiceStatus::Stopped,
            _ => ServiceStatus::Started,
        }
    }
}
