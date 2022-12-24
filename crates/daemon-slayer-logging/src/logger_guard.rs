use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct LoggerGuard {
    guards: Vec<Arc<Box<dyn Send + Sync>>>,
}

impl LoggerGuard {
    pub(crate) fn add_guard(&mut self, guard: Box<dyn Send + Sync>) {
        self.guards.push(Arc::new(guard));
    }
}
