use tracing_appender::non_blocking::WorkerGuard;

use crate::ipc_writer;

pub struct LoggerGuard {
    guards: Vec<WorkerGuard>,
    console_guard: Option<ipc_writer::WorkerGuard>,
}

impl LoggerGuard {
    pub(crate) fn new() -> Self {
        Self {
            guards: vec![],
            console_guard: None,
        }
    }

    pub(crate) fn add_guard(&mut self, guard: WorkerGuard) {
        self.guards.push(guard);
    }

    pub(crate) fn set_console_guard(&mut self, guard: ipc_writer::WorkerGuard) {
        self.console_guard = Some(guard);
    }
}
