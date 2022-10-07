use std::{future::Future, pin::Pin, str::FromStr, sync::Arc};

use tokio::sync::RwLock;
use tokio_cron_scheduler::{
    DataStore, InitStore, JobId, JobIdAndNotification, JobSchedulerError, JobUuid,
    NotificationData, NotificationId, NotificationStore, Uuid,
};

use super::sqlite_store::SqliteStore;

struct SqliteNotificationStore {
    store: Arc<RwLock<SqliteStore>>,
}

impl Default for SqliteNotificationStore {
    fn default() -> Self {
        let store = Arc::new(RwLock::new(SqliteStore::default()));
        Self { store }
    }
}

impl DataStore<NotificationData> for SqliteNotificationStore {
    fn get(
        &mut self,
        id: Uuid,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Option<NotificationData>,
                        tokio_cron_scheduler::JobSchedulerError,
                    >,
                > + Send,
        >,
    > {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::GetJobData),
                SqliteStore::Initialized(store) => {
                    let uuid_str = id.to_string();
                    let notification = sqlx::query!(
                        "
                    SELECT uuid, job_id, extra from job_notification_store
                    WHERE uuid = ?
                ",
                        uuid_str
                    )
                    .fetch_optional(store)
                    .await
                    .unwrap();
                    if notification.is_none() {
                        return Ok(None);
                    }
                    let notification = notification.unwrap();
                    let notification_id = notification.uuid.unwrap_or_default();

                    let job_states = sqlx::query!(
                        "
                    SELECT state from job_state_store where notification_id = ?
                    ",
                        notification_id
                    )
                    .fetch_all(store)
                    .await
                    .unwrap();

                    let job_states = job_states
                        .iter()
                        .map(|j| j.state as i32)
                        .collect::<Vec<_>>();

                    Ok(Some(NotificationData {
                        job_id: Some(JobIdAndNotification {
                            job_id: notification
                                .job_id
                                .map(|j| JobUuid::from(Uuid::from_str(&j).unwrap())),
                            notification_id: Some(JobUuid::from(
                                Uuid::from_str(&notification_id).unwrap(),
                            )),
                        }),
                        job_states,
                        extra: notification.extra.unwrap_or_default(),
                    }))
                }
            }
        })
    }

    fn add_or_update(
        &mut self,
        data: NotificationData,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), tokio_cron_scheduler::JobSchedulerError>>
                + Send,
        >,
    > {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::UpdateJobData),
                SqliteStore::Initialized(store) => {
                    let (job_id, notification_id) =
                        match data.job_id_and_notification_id_from_data() {
                            Some((job_id, notification_id)) => (job_id, notification_id),
                            None => return Err(JobSchedulerError::UpdateJobData),
                        };
                    let notification_id = notification_id.to_string();
                    let job_id = job_id.to_string();
                    sqlx::query!(
                        "
                        DELETE FROM job_state_store WHERE notification_id = ?
                    ",
                        notification_id
                    )
                    .execute(store)
                    .await
                    .unwrap();

                    sqlx::query!(
                        "
                    INSERT INTO job_notification_store(uuid, job_id, extra)
                    VALUES($1, $2, $3)
                    ON CONFLICT(uuid) DO UPDATE
                    SET job_id = $2, extra = $3
                    ",
                        notification_id,
                        job_id,
                        data.extra
                    )
                    .execute(store)
                    .await
                    .unwrap();

                    let sql_text = "
                    INSERT INTO job_state_store(notification_id, state)
                    VALUES
                    "
                    .to_string()
                        + &*data
                            .job_states
                            .iter()
                            .map(|s| format!("($1, {})", s))
                            .collect::<Vec<_>>()
                            .join(",");
                    sqlx::query(&sql_text)
                        .bind(notification_id)
                        .execute(store)
                        .await
                        .unwrap();
                    Ok(())
                }
            }
        })
    }

    fn delete(
        &mut self,
        guid: Uuid,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), tokio_cron_scheduler::JobSchedulerError>>
                + Send,
        >,
    > {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::CantRemove),
                SqliteStore::Initialized(store) => {
                    let uuid = guid.to_string();
                    sqlx::query!(
                        "
                    DELETE FROM job_notification_store WHERE uuid = ?
                    ",
                        uuid
                    )
                    .execute(store)
                    .await
                    .unwrap();
                    Ok(())
                }
            }
        })
    }
}

impl InitStore for SqliteNotificationStore {
    fn init(&mut self) -> Pin<Box<dyn Future<Output = Result<(), JobSchedulerError>> + Send>> {
        let inited = self.inited();
        let store = self.store.clone();

        Box::pin(async move {
            let inited = inited.await;
            if matches!(inited, Ok(false)) || matches!(inited, Err(_)) {
                let mut w = store.write().await;
                let val = w.clone().init().await.unwrap();
                *w = val;
                Ok(())
            } else {
                Ok(())
            }
        })
    }

    fn inited(&mut self) -> Pin<Box<dyn Future<Output = Result<bool, JobSchedulerError>> + Send>> {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            Ok(store.initialized())
        })
    }
}

impl NotificationStore for SqliteNotificationStore {
    fn list_notification_guids_for_job_and_state(
        &mut self,
        job: JobId,
        state: tokio_cron_scheduler::JobNotification,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<NotificationId>, JobSchedulerError>> + Send>> {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            let state = state as i32;
            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::CantListGuids),
                SqliteStore::Initialized(store) => {
                    let job = job.to_string();
                    let rows = sqlx::query!(
                        "
                        SELECT DISTINCT states.uuid
                        FROM job_state_store as st
                        LEFT JOIN job_notification_store as states ON st.notification_id = states.uuid
                        WHERE job_id = $1 AND state = $2
                    ",
                        job,
                        state
                    )
                    .fetch_all(store)
                    .await
                    .unwrap();
                    Ok(rows
                        .iter()
                        .map(|r| {
                            let uuid = r.uuid.clone().unwrap_or_default();
                            Uuid::parse_str(&uuid).unwrap()
                        })
                        .collect::<Vec<_>>())
                }
            }
        })
    }

    fn list_notification_guids_for_job_id(
        &mut self,
        job_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Uuid>, JobSchedulerError>> + Send>> {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;

            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::CantListGuids),
                SqliteStore::Initialized(store) => {
                    let job_id = job_id.to_string();
                    let rows = sqlx::query!(
                        "
                        SELECT DISTINCT uuid FROM job_notification_store WHERE job_id = ?
                    ",
                        job_id
                    )
                    .fetch_all(store)
                    .await
                    .unwrap();
                    Ok(rows
                        .iter()
                        .map(|r| {
                            let uuid = r.uuid.clone().unwrap_or_default();
                            Uuid::parse_str(&uuid).unwrap()
                        })
                        .collect::<Vec<_>>())
                }
            }
        })
    }

    fn delete_notification_for_state(
        &mut self,
        notification_id: Uuid,
        state: tokio_cron_scheduler::JobNotification,
    ) -> Pin<Box<dyn Future<Output = Result<bool, JobSchedulerError>> + Send>> {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            let state = state as i32;
            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::CantListGuids),
                SqliteStore::Initialized(store) => {
                    let state = state as i32;
                    let notification_id = notification_id.to_string();
                    let rows = sqlx::query!(
                        "
                       DELETE FROM job_state_store
                       WHERE notification_id = ? AND state = ?
                       RETURNING state
                    ",
                        notification_id,
                        state
                    )
                    .fetch_all(store)
                    .await
                    .unwrap();
                    Ok(!rows.is_empty())
                }
            }
        })
    }

    fn delete_for_job(
        &mut self,
        job_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), JobSchedulerError>> + Send>> {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;

            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::CantListGuids),
                SqliteStore::Initialized(store) => {
                    let job_id = job_id.to_string();
                    sqlx::query!(
                        "
                       DELETE FROM job_notification_store
                       WHERE job_id = ?
                    ",
                        job_id
                    )
                    .fetch_all(store)
                    .await
                    .unwrap();
                    Ok(())
                }
            }
        })
    }
}
