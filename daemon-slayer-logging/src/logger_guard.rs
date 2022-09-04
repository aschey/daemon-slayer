pub(crate) trait Any {}
impl<T> Any for T {}

pub struct LoggerGuard {
    guards: Vec<Box<dyn Any>>,
}

impl LoggerGuard {
    pub(crate) fn new() -> Self {
        Self { guards: vec![] }
    }

    pub(crate) fn add_guard(&mut self, guard: Box<dyn Any>) {
        self.guards.push(guard);
    }
}
