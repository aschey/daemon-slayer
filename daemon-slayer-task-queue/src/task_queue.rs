use crate::TaskQueueBuilder;
use aide_de_camp::core::DateTime;
use aide_de_camp::prelude::{Decode, Encode, JobError};
use aide_de_camp::prelude::{JobProcessor, Xid};
use aide_de_camp::runner::job_event::JobEvent;
use aide_de_camp::{
    prelude::{Duration, JobRunner, Queue},
    runner::event_store::EventStore,
};
pub use aide_de_camp_sqlite::sqlx::sqlite::SqliteConnectOptions;
use aide_de_camp_sqlite::{
    sqlx::{self, SqlitePool},
    SqliteQueue, SCHEMA_SQL,
};
use tracing::info;

#[derive(Clone)]
pub struct TaskQueue {
    queue: SqliteQueue,
    event_store: EventStore,
}

impl TaskQueue {
    pub async fn new() -> Self {
        Self::from_builder(TaskQueueBuilder::default()).await
    }

    pub async fn builder() -> TaskQueueBuilder {
        TaskQueueBuilder::default()
    }

    pub(crate) async fn from_builder(builder: TaskQueueBuilder) -> Self {
        let pool = SqlitePool::connect_with(builder.sqlite_options)
            .await
            .unwrap();

        sqlx::query(SCHEMA_SQL).execute(&pool).await.unwrap();
        let queue = SqliteQueue::with_pool(pool);

        let mut runner = JobRunner::new(queue.clone(), builder.router, builder.concurrency);

        let event_store = runner.event_store();
        tokio::spawn(async move {
            info!("Running job server");
            runner.run(Duration::seconds(1)).await.unwrap();
        });

        Self { queue, event_store }
    }

    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<JobEvent> {
        self.event_store.subscribe_events()
    }

    pub async fn schedule<J>(&self, payload: J::Payload) -> Xid
    where
        J: JobProcessor + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        self.queue.schedule::<J>(payload).await.unwrap()
    }

    pub async fn schedule_in<J>(&self, payload: J::Payload, scheduled_in: Duration) -> Xid
    where
        J: JobProcessor + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        self.queue
            .schedule_in::<J>(payload, scheduled_in)
            .await
            .unwrap()
    }

    pub async fn schedule_at<J>(&self, payload: J::Payload, scheduled_at: DateTime) -> Xid
    where
        J: JobProcessor + 'static,
        J::Payload: Decode + Encode,
        J::Error: Into<JobError>,
    {
        self.queue
            .schedule_at::<J>(payload, scheduled_at)
            .await
            .unwrap()
    }
}
