mod task_queue;
use daemon_slayer_core::{async_trait, BoxedError};
pub use task_queue::*;
mod task_queue_builder;
pub use aide_de_camp::prelude::{
    CancellationToken, Decode, Encode, JobError, JobProcessor, RunnerOptions, RunnerRouter, Xid,
};
pub use aide_de_camp_sqlite::SqliteQueue;
use daemon_slayer_core::server::{BackgroundService, ServiceContext};
pub use sqlx::sqlite::SqliteConnectOptions;
pub use task_queue_builder::*;

#[async_trait]
impl BackgroundService for TaskQueue {
    fn name<'a>() -> &'a str {
        "task_queue_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        self.run(context.cancellation_token()).await;
        Ok(())
    }
}
