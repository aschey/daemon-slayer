mod task_queue;
pub use task_queue::*;
mod task_queue_builder;
pub use task_queue_builder::*;

pub use aide_de_camp::prelude::{Decode, Encode, JobError, RunnerRouter};
pub use aide_de_camp::prelude::{JobProcessor, ShutdownOptions, Xid};
pub use aide_de_camp::runner::job_event::JobEvent;
pub use aide_de_camp_sqlite::sqlx::sqlite::SqliteConnectOptions;
use daemon_slayer_core::server::EventStore;

pub struct JobEventStore {
    inner: aide_de_camp::runner::event_store::EventStore<JobEvent>,
}

impl JobEventStore {
    fn new(inner: aide_de_camp::runner::event_store::EventStore<JobEvent>) -> Self {
        Self { inner }
    }
}

impl EventStore for JobEventStore {
    type Item = JobEvent;

    fn subscribe_events(&self) -> Box<dyn daemon_slayer_core::server::Receiver<Item = Self::Item>> {
        Box::new(self.inner.subscribe_events())
    }
}

#[async_trait::async_trait]
impl daemon_slayer_core::server::BackgroundService for TaskQueue {
    type Builder = TaskQueueBuilder;
    type Client = TaskQueueClient;

    async fn run_service(builder: Self::Builder) -> Self {
        Self::from_builder(builder).await
    }

    fn get_client(&mut self) -> Self::Client {
        self.client()
    }

    async fn stop(mut self) {
        self.stop().await;
    }
}

impl daemon_slayer_core::server::EventService for TaskQueue {
    type EventStoreImpl = JobEventStore;

    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        JobEventStore::new(self.event_store())
    }
}
