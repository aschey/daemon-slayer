use std::sync::Arc;

use tracing::metadata::LevelFilter;

#[derive(Clone)]
pub struct ReloadHandle {
    reload_fn: Arc<dyn Fn(LevelFilter) + Send + Sync>,
}

impl ReloadHandle {
    pub(crate) fn new(reload_fn: impl Fn(LevelFilter) + Send + Sync + 'static) -> Self {
        Self {
            reload_fn: Arc::new(reload_fn),
        }
    }

    pub fn update_log_level(&self, new_level: LevelFilter) {
        (self.reload_fn)(new_level);
    }
}
