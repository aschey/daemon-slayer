mod task_queue;
use daemon_slayer_core::BoxedError;
pub use task_queue::*;
mod task_queue_builder;
pub use aide_de_camp::prelude::{CancellationToken, JobProcessor, RunnerOptions, Xid};
pub use aide_de_camp::prelude::{Decode, Encode, JobError, RunnerRouter};
use daemon_slayer_core::server::{EventStore, ServiceContext, Stream};
pub use sqlx::sqlite::SqliteConnectOptions;
pub use task_queue_builder::*;

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for TaskQueue {
    type Client = TaskQueueClient;

    fn name<'a>() -> &'a str {
        "task_queue_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        self.run(context.cancellation_token()).await;
        Ok(())
    }

    async fn get_client(&mut self) -> Self::Client {
        self.client()
    }
}
