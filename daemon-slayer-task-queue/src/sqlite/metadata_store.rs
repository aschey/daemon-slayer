use std::{future::Future, pin::Pin, str::FromStr, sync::Arc, time::Duration};

use tokio::sync::RwLock;
use tokio_cron_scheduler::{
    CronJob, CronJobType, DataStore, DateTime, InitStore, JobAndNextTick, JobSchedulerError,
    JobStoredData, JobType, JobUuid, MetaDataStorage, NonCronJob, NonCronJobType, StoredJobType,
    Utc, Uuid,
};

use super::sqlite_store::SqliteStore;
pub(crate) struct SqliteMetadataStore {
    store: Arc<RwLock<SqliteStore>>,
}

fn get_job_type(
    job_type: i32,
    schedule: Option<String>,
    repeating: bool,
    repeated_every: u64,
) -> Option<StoredJobType> {
    let job_type = JobType::from_i32(job_type);
    match job_type {
        Some(JobType::Cron) => schedule.map(|schedule| CronJobType(CronJob { schedule })),
        Some(_) => Some(NonCronJobType(NonCronJob {
            repeating,
            repeated_every,
        })),
        None => None,
    }
}

impl Default for SqliteMetadataStore {
    fn default() -> Self {
        let store = Arc::new(RwLock::new(SqliteStore::default()));
        Self { store }
    }
}

impl DataStore<JobStoredData> for SqliteMetadataStore {
    fn get(
        &mut self,
        id: Uuid,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<JobStoredData>, tokio_cron_scheduler::JobSchedulerError>,
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
                    let row = sqlx::query!(
                        "
                    SELECT uuid, last_updated, next_tick, last_tick, job_type, count, ran, stopped, schedule,
                    repeating, repeated_every, extra from job_metadata_store
                    WHERE uuid = ?
                ",
                        uuid_str
                    ).fetch_one(store).await.unwrap();
                    let data = JobStoredData {
                        id: Some(JobUuid::from(Uuid::from_str(&row.uuid).unwrap_or_default())),
                        last_updated: row.last_updated.map(|l| l as u64),
                        last_tick: row.last_tick.map(|l| l as u64),
                        next_tick: row.next_tick.map(|n| n as u64).unwrap_or_default(),
                        job_type: row.job_type as i32,
                        count: row.count.map(|c| c as u32).unwrap_or_default(),
                        extra: row.extra.unwrap_or_default(),
                        ran: row.ran.unwrap_or_default(),
                        stopped: row.stopped.unwrap_or_default(),
                        job: get_job_type(
                            row.job_type as i32,
                            row.schedule,
                            row.repeating.unwrap_or_default(),
                            row.repeated_every.unwrap_or_default() as u64,
                        ),
                    };
                    Ok(Some(data))
                }
            }
        })
    }

    fn add_or_update(
        &mut self,
        data: JobStoredData,
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
                    let uuid: Uuid = data.id.as_ref().unwrap().into();
                    let uuid_str = uuid.to_string();
                    let last_updated = data.last_updated.as_ref().map(|i| *i as i64);
                    let next_tick = data.next_tick as i64;
                    let schedule = match data.job.as_ref() {
                        Some(CronJobType(ct)) => Some(ct.schedule.clone()),
                        _ => None,
                    };
                    let repeating = match data.job.as_ref() {
                        Some(NonCronJobType(ct)) => Some(ct.repeating),
                        _ => None,
                    };
                    let repeating_every = match data.job.as_ref() {
                        Some(NonCronJobType(ct)) => Some(ct.repeated_every as i64),
                        _ => None,
                    };
                    let last_tick = data.last_tick.as_ref().map(|i| *i as i64);
                    sqlx::query!(
                        "
                    INSERT INTO job_metadata_store(
                        uuid, last_updated, next_tick, job_type, count,
                        ran, stopped, schedule, repeating, repeated_every,
                        extra, last_tick
                    )
                    VALUES (
                        $1, $2, $3, $4, $5, 
                        $6, $7, $8, $9, $10,
                        $11, $12 
                    )
                    ON CONFLICT (uuid) DO UPDATE
                    SET last_updated=$2, next_tick=$3, job_type=$4, count=$5, 
                    ran=$6, stopped=$7, schedule=$8, repeating=$9, repeated_every=$10, 
                    extra=$11, last_tick=$12
                    ",
                        uuid_str,
                        last_updated,
                        next_tick,
                        data.job_type,
                        data.count,
                        data.ran,
                        data.stopped,
                        schedule,
                        repeating,
                        repeating_every,
                        data.extra,
                        last_tick
                    )
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
                    DELETE FROM job_metadata_store WHERE uuid = ?
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

impl InitStore for SqliteMetadataStore {
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

impl MetaDataStorage for SqliteMetadataStore {
    fn list_next_ticks(
        &mut self,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<Vec<tokio_cron_scheduler::JobAndNextTick>, JobSchedulerError>,
                > + Send,
        >,
    > {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::CantListNextTicks),
                SqliteStore::Initialized(store) => {
                    let now = Utc::now().timestamp();
                    let rows = sqlx::query!(
                        "
                    SELECT uuid, job_type, next_tick, last_tick
                    FROM job_metadata_store 
                    WHERE next_tick > 0 AND next_tick < ?
                    ",
                        now,
                    )
                    .fetch_all(store)
                    .await
                    .unwrap();
                    Ok(rows
                        .iter()
                        .map(|row| {
                            let uuid = Uuid::from_str(&row.uuid).unwrap();
                            JobAndNextTick {
                                id: Some(JobUuid::from(uuid)),
                                job_type: row.job_type as i32,
                                next_tick: row.next_tick.unwrap_or_default() as u64,
                                last_tick: row.last_tick.map(|l| l as u64),
                            }
                        })
                        .collect::<Vec<_>>())
                }
            }
        })
    }

    fn set_next_and_last_tick(
        &mut self,
        guid: Uuid,
        next_tick: Option<DateTime<Utc>>,
        last_tick: Option<DateTime<Utc>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), JobSchedulerError>> + Send>> {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::UpdateJobData),
                SqliteStore::Initialized(store) => {
                    let next_tick = next_tick.map(|b| b.timestamp()).unwrap_or(0);
                    let last_tick = last_tick.map(|b| b.timestamp());
                    let uuid = guid.to_string();
                    sqlx::query!(
                        "
                    UPDATE job_metadata_store
                    SET next_tick=?, last_tick=?
                    WHERE uuid=?
                    ",
                        next_tick,
                        last_tick,
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

    fn time_till_next_job(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<std::time::Duration>, JobSchedulerError>> + Send>>
    {
        let store = self.store.clone();
        Box::pin(async move {
            let store = store.read().await;
            match &*store {
                SqliteStore::Created(_) => Err(JobSchedulerError::CouldNotGetTimeUntilNextTick),
                SqliteStore::Initialized(store) => {
                    let now = Utc::now().timestamp();
                    let row = sqlx::query!(
                        "
                    SELECT next_tick
                    FROM job_metadata_store
                    WHERE next_tick > 0 AND next_tick > ?
                    ORDER BY next_tick ASC LIMIT 1
                    ",
                        now
                    )
                    .fetch_one(store)
                    .await
                    .unwrap();
                    let now = now as u64;
                    let next = row.next_tick.unwrap_or_default() as u64;
                    let delta = next - now;
                    Ok(if delta > 0 {
                        Some(Duration::from_secs(next - now))
                    } else {
                        None
                    })
                }
            }
        })
    }
}
