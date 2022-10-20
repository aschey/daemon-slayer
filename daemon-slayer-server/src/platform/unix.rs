use crate::{handler::Handler, service_context::ServiceContext};

pub async fn run_service_main<T: Handler + Send>(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut context = ServiceContext::new();
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
