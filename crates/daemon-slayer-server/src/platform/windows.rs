use crate::{Handler, ServiceError};
use daemon_slayer_core::{
    server::BackgroundServiceManager,
    signal::{self, Signal},
    CancellationToken,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{runtime::Runtime, sync::broadcast};
use tracing::{error, info};
use windows_service::{
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
};

const USER_OWN_PROCESS_TEMPLATE: u32 = 0x50;
const USER_SHARE_PROCESS_TEMPLATE: u32 = 0x60;

pub fn get_service_main<T: Handler>(input_data: Option<T::InputData>) {
    let rt = Runtime::new().expect("Tokio runtime failed to initialize");
    if let Err(e) = rt.block_on(get_service_main_impl::<T>(input_data)) {
        error!("Error running service: {e}");
    }
}

async fn get_service_main_impl<T: Handler>(
    input_data: Option<T::InputData>,
) -> Result<(), ServiceError<T::Error>> {
    set_env_vars::<T>();
    let (signal_tx, _) = broadcast::channel(32);
    signal::set_sender(signal_tx.clone());

    let manager = BackgroundServiceManager::new(CancellationToken::new());
    let handler = T::new(manager.get_context(), input_data)
        .await
        .map_err(|e| ServiceError::ExecutionFailure(e, None))?;

    let windows_service_event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            // Handle stop
            ServiceControl::Stop => {
                info!("Received stop command from service manager");
                if let Err(e) = signal_tx.send(Signal::SIGINT) {
                    error!("Error sending stop signal: {e:?}");
                }
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match service_control_handler::register(
        T::label().application,
        windows_service_event_handler,
    ) {
        Ok(handle) => Arc::new(Mutex::new(handle)),
        Err(e) => {
            return Err(ServiceError::InitializationFailure(
                "Error registering control handler".to_owned(),
                Box::new(e),
            ))?;
        }
    };
    let status_handle_ = status_handle.clone();
    let on_started = move || {
        info!("Setting status to 'running'");
        if let Err(e) = status_handle_
            .lock()
            .unwrap()
            .set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::Running,
                controls_accepted: ServiceControlAccept::STOP,
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })
        {
            error!("Error setting status to 'running': {e:?}");
        }
    };

    let result = handler.run_service(on_started).await;

    let exit_code = match &result {
        Ok(()) => 0,
        Err(_) => 1,
    };

    let background_service_errors = manager.stop().await;

    {
        let handle = status_handle.lock().unwrap();
        handle
            .set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::Win32(exit_code),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })
            .map_err(|e| {
                ServiceError::InitializationFailure(
                    "Error setting status to stopped".to_owned(),
                    Box::new(e),
                )
            })?;
    }

    drop(status_handle);
    ServiceError::from_service_result(result, background_service_errors)
}

fn set_env_vars<T: Handler>() {
    let services_key = registry::Hive::LocalMachine
        .open(
            format!(
                "SYSTEM\\CurrentControlSet\\Services\\{}",
                T::label().application
            ),
            registry::Security::Read,
        )
        .unwrap();

    let is_user_service = matches!(
        services_key.value("Type"),
        Ok(registry::Data::U32(
            USER_OWN_PROCESS_TEMPLATE | USER_SHARE_PROCESS_TEMPLATE
        ))
    );

    // User services don't copy over the environment variables from the template so we need to inject them manually
    if is_user_service {
        if let Ok(registry::Data::MultiString(environment_vars)) = services_key.value("Environment")
        {
            for env_var in environment_vars {
                let var_str = env_var.to_string_lossy();
                let parts = var_str.split('=').collect::<Vec<_>>();
                std::env::set_var(parts[0], parts[1]);
            }
        }
    }
}

pub async fn get_direct_handler<T: Handler>(
    input_data: Option<T::InputData>,
) -> Result<(), ServiceError<T::Error>> {
    let manager = BackgroundServiceManager::new(CancellationToken::new());
    let handler = T::new(manager.get_context(), input_data)
        .await
        .map_err(|e| ServiceError::ExecutionFailure(e, None))?;

    let result = handler.run_service(|| {}).await;
    let background_service_errors = manager.stop().await;
    ServiceError::from_service_result(result, background_service_errors)
}
