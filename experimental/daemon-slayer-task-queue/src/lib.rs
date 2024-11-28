mod task_queue;
mod task_queue_builder;

pub use aide_de_camp::prelude::{
    CancellationToken, Decode, Encode, JobError, JobProcessor, RunnerOptions, RunnerRouter, Xid,
};
pub use aide_de_camp_sqlite::SqliteQueue;
use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::BoxedError;
pub use sqlx::sqlite::SqliteConnectOptions;
pub use task_queue::*;
pub use task_queue_builder::*;

impl BackgroundService for TaskQueue {
    fn name(&self) -> &str {
        "task_queue_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        self.run(context.cancellation_token()).await;
        Ok(())
    }
}
