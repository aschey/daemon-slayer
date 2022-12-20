use std::{path::PathBuf, sync::Arc};

use crate::{AppConfig, Config};
use daemon_slayer_core::{
    server::{
        BackgroundService, BroadcastEventStore, EventService, EventStore, FutureExt,
        ServiceContext, SubsystemHandle,
    },
    BoxedError,
};
use daemon_slayer_file_watcher::FileWatcher;
use futures::stream::StreamExt;

pub struct ConfigClient {}

pub struct ConfigService<T>
where
    T: Config + Default + Send + Sync + Clone + 'static,
{
    config: AppConfig<T>,
    file_tx: tokio::sync::broadcast::Sender<(Arc<T>, Arc<T>)>,
}

impl<T> ConfigService<T>
where
    T: Config + Default + Send + Sync + Clone + 'static,
{
    pub fn new(config: AppConfig<T>) -> Self {
        let (file_tx, _) = tokio::sync::broadcast::channel(32);
        Self { config, file_tx }
    }
}

#[async_trait::async_trait]
impl<T> BackgroundService for ConfigService<T>
where
    T: Config + Default + Send + Sync + Clone + 'static,
{
    type Client = ConfigClient;

    fn name<'a>() -> &'a str {
        "config_service"
    }

    async fn run(mut self, mut context: ServiceContext) -> Result<(), BoxedError> {
        let (_, event_store) = context
            .add_event_service(
                FileWatcher::builder()
                    .with_watch_path(self.config.path())
                    .build(),
            )
            .await;

        let mut event_stream = event_store.subscribe_events();

        while let Ok(Some(_)) = event_stream
            .next()
            .cancel_on_shutdown(&context.get_subsystem_handle())
            .await
        {
            let current = self.config.snapshot();
            self.config.read_config();
            let new = self.config.snapshot();
            self.file_tx.send((current, new)).ok();
        }

        Ok(())
    }

    async fn get_client(&mut self) -> Self::Client {
        ConfigClient {}
    }
}

impl<T> EventService for ConfigService<T>
where
    T: Config + Default + Send + Sync + Clone + 'static,
{
    type EventStoreImpl = BroadcastEventStore<(Arc<T>, Arc<T>)>;

    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        BroadcastEventStore::new(self.file_tx.clone())
    }
}
