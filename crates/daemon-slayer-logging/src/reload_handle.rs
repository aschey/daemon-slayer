use std::sync::Arc;

use tracing::metadata::LevelFilter;

type ReloadFn = Box<dyn Fn(LevelFilter) + Send + Sync>;

#[derive(Clone)]
pub struct ReloadHandle {
    reload_fn: Arc<ReloadFn>,
}

impl ReloadHandle {
    pub(crate) fn new(reload_fn: ReloadFn) -> Self {
        Self {
            reload_fn: Arc::new(reload_fn),
        }
    }

    pub fn update_log_level(&self, new_level: LevelFilter) {
        (self.reload_fn)(new_level);
    }
}
