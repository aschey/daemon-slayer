use super::Signal;

pub trait Client {
    fn add_signal(&self, signal: Signal);
}
