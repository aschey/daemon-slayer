use std::time::Duration;

use daemon_slayer_core::{
    async_trait,
    notify::Notification,
    server::{tokio_stream::StreamExt, BackgroundService, EventStore, ServiceContext},
    BoxedError, FutureExt,
};

#[cfg(feature = "cli")]
pub mod cli;

pub struct NotificationService<E, F> {
    event_store: E,
    create_notification: F,
    shutdown_timeout: Option<Duration>,
}

impl<E, F> NotificationService<E, F>
where
    E: EventStore,
    F: FnMut(E::Item) -> Notification,
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

#[async_trait]
impl<E, F> BackgroundService for NotificationService<E, F>
where
    E: EventStore + Send,
    F: FnMut(E::Item) -> Notification + Send,
{
    fn name(&self) -> &str {
        "notifier_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut event_stream = self.event_store.subscribe_events();
        let cancellation_token = context.cancellation_token();
        while let Ok(Some(event)) = event_stream
            .next()
            .cancel_on_shutdown(&cancellation_token)
            .await
        {
            (self.create_notification)(event).show()?;
        }

        if let Some(timeout) = self.shutdown_timeout {
            if let Ok(Some(event)) = tokio::time::timeout(timeout, event_stream.next()).await {
                (self.create_notification)(event).show()?;
            }
        }

        Ok(())
    }
}
