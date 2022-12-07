mod task_queue;
use std::pin::Pin;

pub use task_queue::*;
mod task_queue_builder;
pub use task_queue_builder::*;

pub use aide_de_camp::prelude::{CancellationToken, JobProcessor, ShutdownOptions, Xid};
pub use aide_de_camp::prelude::{Decode, Encode, JobError, RunnerRouter};
pub use aide_de_camp::runner::job_event::JobEvent;
pub use aide_de_camp_sqlite::sqlx::sqlite::SqliteConnectOptions;
use daemon_slayer_core::server::{EventStore, ServiceContext, Stream, SubsystemHandle};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

pub struct JobEventStore {
    inner: aide_de_camp::runner::event_store::EventStore<JobEvent>,
}

impl JobEventStore {
    fn new(inner: aide_de_camp::runner::event_store::EventStore<JobEvent>) -> Self {
        Self { inner }
    }
}

impl EventStore for JobEventStore {
    type Item = Result<JobEvent, BroadcastStreamRecvError>;

    fn subscribe_events(&self) -> Pin<Box<dyn Stream<Item = Self::Item> + Send>> {
        Box::pin(BroadcastStream::new(self.inner.subscribe_events()))
    }
}

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

impl daemon_slayer_core::server::EventService for TaskQueue {
    type EventStoreImpl = JobEventStore;

    fn get_event_store(&mut self) -> Self::EventStoreImpl {
        JobEventStore::new(self.event_store())
    }
}
