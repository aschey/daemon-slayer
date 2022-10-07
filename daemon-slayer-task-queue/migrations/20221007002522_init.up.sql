-- Add up migration script here
-- Add migration script here
CREATE TABLE IF NOT EXISTS job_metadata_store (
    job_metadata_store_id INTEGER PRIMARY KEY NOT NULL,
    uuid TEXT UNIQUE NOT NULL,
    last_updated INTEGER,
    next_tick INTEGER,
    last_tick INTEGER,
    job_type INTEGER NOT NULL,
    count INTEGER,
    ran BOOLEAN,
    stopped BOOLEAN,
    schedule TEXT,
    repeating BOOLEAN,
    repeated_every INTEGER,
    extra BLOB
);

CREATE TABLE IF NOT EXISTS job_notification_store (
    job_notification_store_id INTEGER PRIMARY KEY NOT NULL,
    uuid TEXT UNIQUE NOT NULL,
    job_id TEXT,
    extra BLOB,
    FOREIGN KEY(job_id) REFERENCES job_metadata_store(uuid) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS job_state_store (
     job_state_store_id INTEGER PRIMARY KEY NOT NULL,
     notification_id TEXT UNIQUE,
     state INTEGER NOT NULL,
     FOREIGN KEY (notification_id) REFERENCES job_notification_store(uuid) ON DELETE CASCADE
)