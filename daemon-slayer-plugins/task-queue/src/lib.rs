mod task_queue;
use std::pin::Pin;

pub use task_queue::*;
mod task_queue_builder;
pub use task_queue_builder::*;

pub use aide_de_camp::prelude::{CancellationToken, JobProcessor, RunnerOptions, Xid};
pub use aide_de_camp::prelude::{Decode, Encode, JobError, RunnerRouter};
use daemon_slayer_core::server::{EventStore, ServiceContext, Stream, SubsystemHandle};
pub use sqlx::sqlite::SqliteConnectOptions;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for TaskQueue {
    type Client = TaskQueueClient;

    async fn run(self, context: ServiceContext) {
        self.run(context.get_subsystem_handle()).await;
    }

    async fn get_client(&mut self) -> Self::Client {
        self.client()
    }
}
