#[cfg(feature = "async-tokio")]
type ServiceContextAsync = crate::ServiceContext;
#[cfg(feature = "blocking")]
type ServiceContextSync = crate::blocking::ServiceContext;

#[maybe_async_cfg::maybe(
    idents(Handler, ServiceContext),
    sync(feature = "blocking"),
    async(feature = "async-tokio")
)]
pub async fn run_service_main<T: crate::handler::Handler + Send>(
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
