use tracing_appender::non_blocking::WorkerGuard;

pub struct LoggerGuard {
    guards: Vec<WorkerGuard>,
}

impl LoggerGuard {
    pub(crate) fn new() -> Self {
        Self { guards: vec![] }
    }

    pub(crate) fn add_guard(&mut self, guard: WorkerGuard) {
        self.guards.push(guard);
    }
}
