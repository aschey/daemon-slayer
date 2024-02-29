use daemon_slayer_core::server::background_service::BackgroundServiceManager;
use daemon_slayer_core::CancellationToken;
#[cfg_attr(target_os = "macos", allow(unused_imports))]
use tap::TapFallible;
#[cfg_attr(target_os = "macos", allow(unused_imports))]
use tracing::{error, warn};

use crate::{Handler, ServiceError};

pub async fn run_as_service<T: Handler>(
    input_data: Option<T::InputData>,
) -> Result<(), ServiceError<T::Error>> {
    let manager =
        BackgroundServiceManager::new(CancellationToken::new(), T::background_service_settings());

    let handler = T::new(manager.get_context(), input_data)
        .await
        .map_err(|e| ServiceError::ExecutionFailure(e, None))?;

    let result = handler
        .run_service(|| {
            #[cfg(target_os = "linux")]
            sd_notify::notify(false, &[sd_notify::NotifyState::Ready])
                .tap_err(|e| error!("Error sending ready notification: {e:?}"))
                .ok();
        })
        .await;

    #[cfg(target_os = "linux")]
    sd_notify::notify(false, &[sd_notify::NotifyState::Stopping])
        .tap_err(|e| warn!("Error sending stopping notification: {e:?}"))
        .ok();

    let background_service_errors = manager.cancel().await;
    ServiceError::from_service_result(result, background_service_errors)
}
