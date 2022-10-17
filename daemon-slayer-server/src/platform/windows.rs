use daemon_slayer_plugin_signals::Signal;
use std::error::Error;
use tracing::{error, info};

const USER_OWN_PROCESS_TEMPLATE: u32 = 0x50;
const USER_SHARE_PROCESS_TEMPLATE: u32 = 0x60;

#[cfg(feature = "async-tokio")]
pub fn get_service_main_async<T: crate::Handler + Send>() {
    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime failed to initialize");
    rt.block_on(get_service_main_impl_async::<T>())
}

#[cfg(feature = "blocking")]
pub fn get_service_main_sync<T: crate::blocking::Handler + Send>() {
    get_service_main_impl_sync::<T>()
}

#[cfg(feature = "async-tokio")]
type ServiceContextAsync = crate::ServiceContext;
#[cfg(feature = "blocking")]
type ServiceContextSync = crate::blocking::ServiceContext;

#[cfg(feature = "async-tokio")]
type SignalHandlerAsync = daemon_slayer_plugin_signals::SignalHandler;
#[cfg(feature = "blocking")]
type SignalHandlerSync = daemon_slayer_plugin_signals::blocking::SignalHandler;

#[maybe_async_cfg::maybe(
    idents(
        Handler,
        ServiceContext,
        SignalHandler,
        set_env_vars(snake),
        get_channel(snake),
        start_file_watcher(snake),
        start_event_loop(snake),
        send_stop_signal(snake),
        join_handle(snake),
        get_context(snake),
    ),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
async fn get_service_main_impl<T: crate::handler::Handler + Send>() {
    set_env_vars::<T>();
    let signal_tx = get_channel();
    SignalHandler::set_sender(signal_tx.clone());

    let mut context = ServiceContext::new();
    let handler = T::new(&mut context).await;

    let windows_service_event_handler = move |control_event| -> crate::windows_service::service_control_handler::ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            crate::windows_service::service::ServiceControl::Interrogate => crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError,

            // Handle stop
            crate::windows_service::service::ServiceControl::Stop => {
                if let Err(e) = send_stop_signal(&signal_tx) {
                    error!("Error sending stop signal: {e:?}");
                }
                crate::windows_service::service_control_handler::ServiceControlHandlerResult::NoError
            }

            _ => crate::windows_service::service_control_handler::ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match crate::windows_service::service_control_handler::register(
        T::get_service_name(),
        windows_service_event_handler,
    ) {
        Ok(handle) => std::sync::Arc::new(std::sync::Mutex::new(handle)),
        Err(e) => {
            error!("Error registering control handler {e}");
            return;
        }
    };
    let status_handle_ = status_handle.clone();
    let on_started = move || {
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

    context.stop().await;

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
}

#[maybe_async_cfg::maybe(
    idents(Handler,),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
fn set_env_vars<T: crate::handler::Handler + Send>() {
    let services_key = registry::Hive::LocalMachine
        .open(
            format!(
                "SYSTEM\\CurrentControlSet\\Services\\{}",
                T::get_service_name()
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

#[cfg(feature = "async-tokio")]
fn get_channel_async() -> tokio::sync::broadcast::Sender<daemon_slayer_plugin_signals::Signal> {
    let (tx, _) = tokio::sync::broadcast::channel(32);
    tx
}

#[cfg(feature = "blocking")]
fn get_channel_sync(
) -> std::sync::Arc<std::sync::Mutex<bus::Bus<daemon_slayer_plugin_signals::Signal>>> {
    std::sync::Arc::new(std::sync::Mutex::new(bus::Bus::new(32)))
}

#[cfg(feature = "async-tokio")]
fn send_stop_signal_async(
    event_tx: &tokio::sync::broadcast::Sender<Signal>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    event_tx.send(Signal::SIGINT)?;

    Ok(())
}

#[cfg(feature = "blocking")]
fn send_stop_signal_sync(
    event_tx: &std::sync::Arc<std::sync::Mutex<bus::Bus<Signal>>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    event_tx.lock().unwrap().broadcast(Signal::SIGINT);
    Ok(())
}

#[maybe_async_cfg::maybe(
    idents(Handler, ServiceContext, SignalHandler, get_channel(snake)),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
pub async fn get_direct_handler<T: crate::handler::Handler + Send>(
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut context = ServiceContext::new();
    let handler = T::new(&mut context).await;

    handler.run_service(|| {}).await?;
    context.stop().await;
    Ok(())
}
