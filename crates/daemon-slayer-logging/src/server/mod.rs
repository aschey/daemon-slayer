use crate::{LoggerGuard, UserConfig};
use daemon_slayer_core::{
    server::{
        tokio_stream::StreamExt, BackgroundService, BroadcastEventStore, EventStore, ServiceContext,
    },
    BoxedError, FutureExt,
};
use std::{ops::Deref, sync::Arc};

pub trait LoggingConfig: AsRef<UserConfig> + Send + Sync + 'static {}
impl<T> LoggingConfig for T where T: AsRef<UserConfig> + Send + Sync + 'static {}

pub struct LoggingUpdateService<T: LoggingConfig> {
    file_events: BroadcastEventStore<(Arc<T>, Arc<T>)>,
    guard: LoggerGuard,
}

impl<T: LoggingConfig> LoggingUpdateService<T> {
    pub fn new(guard: LoggerGuard, file_events: BroadcastEventStore<(Arc<T>, Arc<T>)>) -> Self {
        Self { guard, file_events }
    }
}

#[async_trait::async_trait]
impl<T: LoggingConfig> BackgroundService for LoggingUpdateService<T> {
    fn name<'a>() -> &'a str {
        "logging_update_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut rx = self.file_events.subscribe_events();
        while let Ok(Some(Ok((_, new)))) = rx
            .next()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            let log_level = new.deref().as_ref().log_level.to_level_filter();

            self.guard.update_log_level(log_level);
        }
        Ok(())
    }
}
