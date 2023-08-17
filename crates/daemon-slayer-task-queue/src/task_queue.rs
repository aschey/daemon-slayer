use aide_de_camp::prelude::{Duration, JobRunner};
use aide_de_camp_sqlite::{SqliteQueue, MIGRATOR};
use daemon_slayer_core::CancellationToken;
use sqlx::SqlitePool;
use tracing::info;

use crate::TaskQueueBuilder;

pub struct TaskQueue {
    queue: SqliteQueue,
    runner: JobRunner<SqliteQueue>,
}

impl TaskQueue {
    pub async fn new() -> Self {
        Self::from_builder(TaskQueueBuilder::default()).await
    }

    pub fn builder() -> TaskQueueBuilder {
        TaskQueueBuilder::default()
    }

    pub fn get_client(&self) -> SqliteQueue {
        self.queue.clone()
    }

    pub(crate) async fn from_builder(builder: TaskQueueBuilder) -> Self {
        let pool = SqlitePool::connect_with(builder.sqlite_options)
            .await
            .unwrap();

        MIGRATOR.run(&pool).await.unwrap();
        let queue = SqliteQueue::with_pool(pool);

        let runner = JobRunner::new(
            queue.clone(),
            builder.router,
            builder.concurrency,
            Default::default(),
        );

        Self { queue, runner }
    }

    pub async fn run(mut self, cancellation_token: CancellationToken) {
        info!("Running job server");
        self.runner
            .run_with_shutdown(
                Duration::seconds(1),
                Box::pin(async move { cancellation_token.cancelled().await }),
            )
            .await
            .unwrap();
    }
}
