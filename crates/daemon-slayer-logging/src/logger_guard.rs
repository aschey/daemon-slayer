use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct LoggerGuard {
    guards: Vec<Arc<dyn Send + Sync>>,
}

impl LoggerGuard {
    pub(crate) fn add_guard(&mut self, guard: impl Send + Sync + 'static) {
        self.guards.push(Arc::new(guard));
    }
}
