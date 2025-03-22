use std::time::Duration;

use daemon_slayer_core::notify::AsyncNotification;
use daemon_slayer_core::server::EventStore;
use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::tokio_stream::StreamExt;
use daemon_slayer_core::{BoxedError, FutureExt};
use tap::TapFallible;
use tracing::error;

#[cfg(feature = "dialog")]
pub mod dialog;
#[cfg(feature = "native-notification")]
pub mod notification;

pub struct NotificationService<E, F> {
    event_store: E,
    create_notification: F,
    shutdown_timeout: Option<Duration>,
}

impl<E, F, N> NotificationService<E, F>
where
    E: EventStore,
    F: FnMut(E::Item) -> Option<N>,
    N: AsyncNotification,
{
    pub fn new(event_store: E, create_notification: F) -> Self {
        Self {
            event_store,
            create_notification,
            shutdown_timeout: None,
        }
    }

    pub fn with_shutdown_timeout(self, timeout: Duration) -> Self {
        Self {
            shutdown_timeout: Some(timeout),
            ..self
        }
    }
}

impl<E, F, N> BackgroundService for NotificationService<E, F>
where
    E: EventStore + Send,
    F: FnMut(E::Item) -> Option<N> + Send,
    N: AsyncNotification + Send + Sync,
{
    fn name(&self) -> &str {
        "notifier_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut event_stream = self.event_store.subscribe_events();
        while let Ok(Some(event)) = event_stream.next().cancel_with(context.cancelled()).await {
            if let Some(notification) = (self.create_notification)(event) {
                notification
                    .show()
                    .await
                    .tap_err(|e| error!("Error showing notification: {e}"))
                    .ok();
            }
        }

        if let Some(timeout) = self.shutdown_timeout {
            if let Ok(Some(event)) = tokio::time::timeout(timeout, event_stream.next()).await {
                if let Some(notification) = (self.create_notification)(event) {
                    notification
                        .show()
                        .await
                        .tap_err(|e| error!("Error showing notification: {e}"))
                        .ok();
                }
            }
        }

        Ok(())
    }
}
