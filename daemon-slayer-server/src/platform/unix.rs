use daemon_slayer_core::server::{SubsystemHandle, Toplevel};
use std::time::Duration;

use crate::{handler::Handler, service_context::ServiceContext};

pub async fn run_service_main<T: Handler + Send + 'static>(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Toplevel::new()
        .start("service_main", run_subsys::<T>)
        .handle_shutdown_requests(Duration::from_millis(5000))
        .await?;
    Ok(())
}

async fn run_subsys<T: Handler + Send + 'static>(
    subsys: SubsystemHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut context = ServiceContext::new(subsys);
    let handler = T::new(&mut context).await;

    let result = handler
        .run_service(|| {
            #[cfg(target_os = "linux")]
            crate::sd_notify::notify(false, &[crate::sd_notify::NotifyState::Ready]).unwrap();
        })
        .await;

    #[cfg(target_os = "linux")]
    sd_notify::notify(false, &[crate::sd_notify::NotifyState::Stopping]).unwrap();

    context.stop().await;
    result
}
