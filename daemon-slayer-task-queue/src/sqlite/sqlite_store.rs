use std::{future::Future, pin::Pin};

use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, Pool, Sqlite};
use tokio_cron_scheduler::JobSchedulerError;

static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Clone)]
pub(crate) enum SqliteStore {
    Created(String),
    Initialized(Pool<Sqlite>),
}

impl Default for SqliteStore {
    fn default() -> Self {
        let db_url = std::env::var("DATABASE_URL").unwrap();
        Self::Created(db_url)
    }
}
impl SqliteStore {
    pub fn initialized(&self) -> bool {
        matches!(self, Self::Initialized(_))
    }

    pub fn init(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<SqliteStore, JobSchedulerError>> + Send>> {
        Box::pin(async move {
            match self {
                Self::Created(url) => {
                    let writer_opts = SqliteConnectOptions::new()
                        .filename(url)
                        .create_if_missing(true);
                    //.log_statements(LevelFilter::Debug)
                    // .log_slow_statements(LevelFilter::Info, Duration::from_secs(1))
                    // .to_owned();

                    let write_pool = sqlx::pool::PoolOptions::new()
                        .max_connections(1)
                        .connect_with(writer_opts)
                        .await
                        .unwrap();
                    MIGRATOR.run(&write_pool).await.unwrap();
                    Ok(Self::Initialized(write_pool))
                }
                Self::Initialized(pool) => Ok(Self::Initialized(pool)),
            }
        })
    }
}
