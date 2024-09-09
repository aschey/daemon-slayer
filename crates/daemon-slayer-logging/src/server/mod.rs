use std::ops::Deref;
use std::sync::Arc;

use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::tokio_stream::StreamExt;
use daemon_slayer_core::server::{BroadcastEventStore, EventStore};
use daemon_slayer_core::{BoxedError, FutureExt};

use crate::{ReloadHandle, UserConfig};

pub trait LoggingConfig: AsRef<UserConfig> + Send + Sync + 'static {}
impl<T> LoggingConfig for T where T: AsRef<UserConfig> + Send + Sync + 'static {}

pub struct LoggingUpdateService<T: LoggingConfig> {
    file_events: BroadcastEventStore<(Arc<T>, Arc<T>)>,
    reload_handle: ReloadHandle,
}

impl<T: LoggingConfig> LoggingUpdateService<T> {
    pub fn new(
        reload_handle: ReloadHandle,
        file_events: BroadcastEventStore<(Arc<T>, Arc<T>)>,
    ) -> Self {
        Self {
            reload_handle,
            file_events,
        }
    }
}

impl<T: LoggingConfig> BackgroundService for LoggingUpdateService<T> {
    fn name(&self) -> &str {
        "logging_update_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut rx = self.file_events.subscribe_events();
        while let Ok(Some(Ok((_, new)))) = rx
            .next()
            .cancel_with(context.cancelled())
            .await
        {
            let log_level = new.deref().as_ref().log_level.to_level_filter();

            self.reload_handle.update_log_level(log_level);
        }
        Ok(())
    }
}
