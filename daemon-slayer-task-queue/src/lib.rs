use sqlite::{metadata_store::SqliteMetadataStore, notification_store::SqliteNotificationStore};
pub use tokio_cron_scheduler::JobScheduler;
use tokio_cron_scheduler::{SimpleJobCode, SimpleNotificationCode};
mod sqlite;

pub fn create_scheduler() -> JobScheduler {
    let metadata_storage = Box::new(SqliteMetadataStore::default());
    let notification_storage = Box::new(SqliteNotificationStore::default());

    let simple_job_code = Box::new(SimpleJobCode::default());
    let simple_notification_code = Box::new(SimpleNotificationCode::default());

    JobScheduler::new_with_storage_and_code(
        metadata_storage,
        notification_storage,
        simple_job_code,
        simple_notification_code,
    )
    .unwrap()
}
