pub struct ServiceConfig {
    #[cfg(feature = "task-queue")]
    pub(crate) router: daemon_slayer_task_queue::RunnerRouter,
}

impl ServiceConfig {
    #[cfg(feature = "task-queue")]
    pub fn add_job_handler<J>(&mut self, job: J)
    where
        J: daemon_slayer_task_queue::JobProcessor + 'static,
        J::Payload: daemon_slayer_task_queue::Decode + daemon_slayer_task_queue::Encode,
        J::Error: Into<daemon_slayer_task_queue::JobError>,
    {
        self.router.add_job_handler(job);
    }
}
