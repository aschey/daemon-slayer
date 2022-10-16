use crate::TaskQueueBuilder;
use aide_de_camp::core::DateTime;
use aide_de_camp::prelude::{Decode, Encode, JobError, QueueError};
use aide_de_camp::prelude::{JobProcessor, ShutdownOptions, Xid};
use aide_de_camp::runner::job_event::JobEvent;
use aide_de_camp::{
    prelude::{Duration, JobRunner, Queue},
    runner::event_store::EventStore,
};
pub use aide_de_camp_sqlite::sqlx::sqlite::SqliteConnectOptions;
use aide_de_camp_sqlite::{sqlx::SqlitePool, SqliteQueue, MIGRATOR};
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
    event_store: EventStore<JobEvent>,
    stop_tx: tokio::sync::mpsc::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

impl TaskQueue {
    pub async fn new() -> Self {
        Self::from_builder(TaskQueueBuilder::default()).await
    }

    pub async fn builder() -> TaskQueueBuilder {
        TaskQueueBuilder::default()
    }

    pub async fn stop(self) {
        self.stop_tx.send(()).await.unwrap();
        self.handle.await.unwrap();
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

        let mut runner = JobRunner::new(queue.clone(), builder.router, builder.concurrency);

        let event_store = runner.event_store();
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(32);

        let handle = tokio::spawn(async move {
            info!("Running job server");
            runner
                .run_with_shutdown(
                    Duration::seconds(1),
                    Box::pin(async move {
                        stop_rx.recv().await;
                    }),
                    ShutdownOptions::default(),
                )
                .await
                .unwrap();
        });

        Self {
            queue,
            event_store,
            stop_tx,
            handle,
        }
    }

    pub fn event_store(&self) -> EventStore<JobEvent> {
        self.event_store.clone()
    }
}
