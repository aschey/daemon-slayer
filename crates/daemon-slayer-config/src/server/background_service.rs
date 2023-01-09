use crate::{AppConfig, Configurable};
use daemon_slayer_core::{
    async_trait,
    server::{BackgroundService, BroadcastEventStore, EventStore, ServiceContext},
    BoxedError, FutureExt,
};
use daemon_slayer_file_watcher::FileWatcher;
use futures::stream::StreamExt;
use std::sync::Arc;
use tap::TapFallible;
use tokio::sync::broadcast;
use tracing::error;

pub struct ConfigService<T>
where
    T: Configurable,
{
    config: AppConfig<T>,
    file_tx: broadcast::Sender<(Arc<T>, Arc<T>)>,
}

impl<T> ConfigService<T>
where
    T: Configurable,
{
    pub fn new(config: AppConfig<T>) -> Self {
        let (file_tx, _) = broadcast::channel(32);
        Self { config, file_tx }
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<(Arc<T>, Arc<T>)> {
        BroadcastEventStore::new(self.file_tx.clone())
    }
}

#[async_trait]
impl<T> BackgroundService for ConfigService<T>
where
    T: Configurable,
{
    fn name<'a>() -> &'a str {
        "config_service"
    }

    async fn run(mut self, mut context: ServiceContext) -> Result<(), BoxedError> {
        let file_watcher = FileWatcher::builder()
            .with_watch_path(self.config.full_path())
            .build();
        let event_store = file_watcher.get_event_store();
        context.add_service(file_watcher).await?;

        let mut event_stream = event_store.subscribe_events();

        while let Ok(Some(_)) = event_stream
            .next()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            let current = self.config.snapshot();
            if self.config.read_config().tap_err(|e| error!("{e}")).is_ok() {
                let new = self.config.snapshot();
                self.file_tx.send((current, new)).ok();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "./background_service_test.rs"]
mod background_service_test;
