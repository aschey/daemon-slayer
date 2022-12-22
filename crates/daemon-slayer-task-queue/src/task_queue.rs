use crate::TaskQueueBuilder;
use aide_de_camp::core::DateTime;
use aide_de_camp::prelude::{Decode, Encode, JobError, QueueError};
use aide_de_camp::prelude::{Duration, JobRunner, Queue};
use aide_de_camp::prelude::{JobProcessor, Xid};
use aide_de_camp_sqlite::{SqliteQueue, MIGRATOR};
use daemon_slayer_core::CancellationToken;
use sqlx::SqlitePool;
use tracing::info;

#[derive(Clone)]
pub struct TaskQueueClient {
    queue: SqliteQueue,
}

impl TaskQueueClient {
    pub async fn schedule<J>(&self, payload: J::Payload, priority: i8) -> Xid
    where
        J: JobProcessor + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        self.queue.schedule::<J>(payload, priority).await.unwrap()
    }

    pub async fn schedule_in<J>(
        &self,
        payload: J::Payload,
        scheduled_in: Duration,
        priority: i8,
    ) -> Xid
    where
        J: JobProcessor + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        self.queue
            .schedule_in::<J>(payload, scheduled_in, priority)
            .await
            .unwrap()
    }

    pub async fn schedule_at<J>(
        &self,
        payload: J::Payload,
        scheduled_at: DateTime,
        priority: i8,
    ) -> Xid
    where
        J: JobProcessor + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        self.queue
            .schedule_at::<J>(payload, scheduled_at, priority)
            .await
            .unwrap()
    }

    pub async fn cancel_job(&self, job_id: Xid) -> Result<(), QueueError> {
        self.queue.cancel_job(job_id).await
    }

    pub async fn unschedule_job<J>(&self, job_id: Xid) -> Result<J::Payload, QueueError>
    where
        J: JobProcessor + 'static,
        J::Payload: Decode,
    {
        self.queue.unschedule_job::<J>(job_id).await
    }
}

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

    pub fn client(&self) -> TaskQueueClient {
        TaskQueueClient {
            queue: self.queue.clone(),
        }
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
