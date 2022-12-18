use daemon_slayer_core::{
    server::{ServiceManager, SubsystemHandle, Toplevel},
    signal::{self, Signal},
    AsAny,
};
use std::{any::Any, error::Error, time::Duration};
use tracing::{error, info};

const USER_OWN_PROCESS_TEMPLATE: u32 = 0x50;
const USER_SHARE_PROCESS_TEMPLATE: u32 = 0x60;

pub fn get_service_main<T: crate::Handler + Send + 'static>(input_data: Option<T::InputData>) {
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime failed to initialize");
    if let Err(e) = rt.block_on(get_service_main_impl::<T>(input_data)) {
        error!("{e}");
    }
}

async fn get_service_main_impl<T: crate::handler::Handler + Send + 'static>(
    input_data: Option<T::InputData>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Toplevel::new()
        .start("service_main", |subsys| run_subsys::<T>(subsys, input_data))
        .handle_shutdown_requests(Duration::from_millis(5000))
        .await?;
    Ok(())
}

async fn run_subsys<T: crate::handler::Handler + Send + 'static>(
    subsys: SubsystemHandle,
    input_data: Option<T::InputData>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    set_env_vars::<T>();
    let (signal_tx, _) = tokio::sync::broadcast::channel(32);
    signal::set_sender(signal_tx.clone());

    let manager = ServiceManager::new(subsys);
    let handler = T::new(manager.get_context().await, input_data).await;

    let windows_service_event_handler = move |control_event| -> crate::windows_service::service_control_handler::ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            crate::windows_service::service::ServiceControl::Interrogate => crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError,

            // Handle stop
            crate::windows_service::service::ServiceControl::Stop => {
                info!("Received stop command from service manager");
                if let Err(e) = signal_tx.send(Signal::SIGINT) {
                    error!("Error sending stop signal: {e:?}");
                }
                crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError
            }

            _ => crate::windows_service::service_control_handler::ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match windows_service::service_control_handler::register(
        T::label().application,
        windows_service_event_handler,
    ) {
        Ok(handle) => std::sync::Arc::new(std::sync::Mutex::new(handle)),
        Err(e) => {
            return Err(format!("Error registering control handler {e}"))?;
        }
    };
    let status_handle_ = status_handle.clone();
    let on_started = move || {
        info!("Setting status to 'running'");
        if let Err(e) = status_handle_.lock().unwrap().set_service_status(
            crate::windows_service::service::ServiceStatus {
                service_type: crate::windows_service::service::ServiceType::OWN_PROCESS,
                current_state: crate::windows_service::service::ServiceState::Running,
                controls_accepted: crate::windows_service::service::ServiceControlAccept::STOP,
                exit_code: crate::windows_service::service::ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: std::time::Duration::default(),
                process_id: None,
            },
        ) {
            error!("Error setting status to 'running': {e:?}");
        }
    };

    let service_result = handler.run_service(on_started).await;

    let exit_code = match service_result {
        Ok(()) => 0,
        Err(e) => {
            error!("Service exited with error: {e}");
            1
        }
    };

    manager.stop().await;

    {
        let handle = status_handle.lock().unwrap();
        if let Err(e) = handle.set_service_status(crate::windows_service::service::ServiceStatus {
            service_type: crate::windows_service::service::ServiceType::OWN_PROCESS,
            current_state: crate::windows_service::service::ServiceState::Stopped,
            controls_accepted: crate::windows_service::service::ServiceControlAccept::empty(),
            exit_code: crate::windows_service::service::ServiceExitCode::Win32(exit_code),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        }) {
            error!("Error setting status to stopped: {e:?}");
        }
    }

    drop(status_handle);
    Ok(())
}

fn set_env_vars<T: crate::handler::Handler + Send>() {
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

pub async fn get_direct_handler<T: crate::handler::Handler + Send + 'static>(
    input_data: Option<T::InputData>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    Toplevel::new()
        .start("service_main", |subsys| {
            direct_subsys::<T>(subsys, input_data)
        })
        .handle_shutdown_requests(Duration::from_millis(5000))
        .await?;
    Ok(())
}

async fn direct_subsys<T: crate::handler::Handler + Send + 'static>(
    subsys: SubsystemHandle,
    input_data: Option<T::InputData>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = ServiceManager::new(subsys);
    let handler = T::new(manager.get_context().await, input_data).await;

    handler.run_service(|| {}).await?;
    manager.stop().await;
    Ok(())
}
