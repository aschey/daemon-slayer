use std::sync::Arc;

use tracing::metadata::LevelFilter;

#[derive(Clone, Default)]
pub struct LoggerGuard {
    guards: Vec<Arc<Box<dyn Send + Sync>>>,
    reload_handle: Option<Arc<Box<dyn Fn(LevelFilter) + Send + Sync>>>,
}

impl LoggerGuard {
    pub(crate) fn add_guard(&mut self, guard: Box<dyn Send + Sync>) {
        self.guards.push(Arc::new(guard));
    }

    pub(crate) fn set_reload_handle(&mut self, handle: Box<dyn Fn(LevelFilter) + Send + Sync>) {
        self.reload_handle = Some(Arc::new(handle));
    }

    pub fn update_log_level(&self, new_level: LevelFilter) {
        if let Some(handle) = &self.reload_handle {
            handle(new_level);
        }
    }
}
