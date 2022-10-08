#[cfg(feature = "async-tokio")]
pub struct ServiceContextAsync {
    #[cfg(feature = "task-queue")]
    pub task_queue: daemon_slayer_task_queue::TaskQueue,
}

#[cfg(feature = "blocking")]
pub struct ServiceContextSync {}
