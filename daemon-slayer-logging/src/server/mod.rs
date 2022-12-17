use std::{ops::Deref, sync::Arc};

use daemon_slayer_core::{
    config::Accessor,
    server::{
        tokio_stream::StreamExt, BackgroundService, BroadcastEventStore, EventStore, ServiceContext,
    },
};

use crate::{LoggerGuard, UserConfig};

pub struct LoggingUpdateService<T: AsRef<UserConfig> + Send + Sync + 'static> {
    file_events: BroadcastEventStore<(Arc<T>, Arc<T>)>,
    guard: LoggerGuard,
}

impl<T: AsRef<UserConfig> + Send + Sync + 'static> LoggingUpdateService<T> {
    pub fn new(guard: LoggerGuard, file_events: BroadcastEventStore<(Arc<T>, Arc<T>)>) -> Self {
        Self { guard, file_events }
    }
}

#[async_trait::async_trait]
impl<T: AsRef<UserConfig> + Send + Sync + 'static> BackgroundService for LoggingUpdateService<T> {
    type Client = ();

    async fn run(mut self, _context: ServiceContext) {
        let mut rx = self.file_events.subscribe_events();
        while let Some(Ok((_, new))) = rx.next().await {
            let log_level = new.deref().as_ref().log_level.to_level_filter();

            self.guard.update_log_level(log_level);
        }
    }

    async fn get_client(&mut self) -> Self::Client {}
}
