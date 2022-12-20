use crate::Handler;
use daemon_slayer_core::{
    server::{ServiceManager, SubsystemHandle, Toplevel},
    BoxedError,
};
use tap::TapFallible;
use tracing::error;

pub async fn run_as_service<T: Handler + Send + 'static>(
    input_data: Option<T::InputData>,
) -> Result<(), BoxedError> {
    Toplevel::new()
        .start("service_main", |subsys| run_subsys::<T>(subsys, input_data))
        .handle_shutdown_requests(T::shutdown_timeout())
        .await?;
    Ok(())
}

async fn run_subsys<T: Handler + Send + 'static>(
    subsys: SubsystemHandle,
    input_data: Option<T::InputData>,
) -> Result<(), BoxedError> {
    let manager = ServiceManager::new(subsys);
    let handler = T::new(manager.get_context().await, input_data).await;

    let result = handler
        .run_service(|| {
            #[cfg(target_os = "linux")]
            sd_notify::notify(false, &[crate::sd_notify::NotifyState::Ready])
                .tap_err(|e| error!("Error sending ready notification: {e:?}"))
                .ok();
        })
        .await;

    #[cfg(target_os = "linux")]
    sd_notify::notify(false, &[crate::sd_notify::NotifyState::Stopping])
        .tap_err(|e| error!("Error sending stopping notification: {e:?}"))
        .ok();

    let errors = manager.stop().await;
    result
}
