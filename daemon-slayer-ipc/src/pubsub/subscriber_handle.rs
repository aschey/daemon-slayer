use futures::stream::AbortHandle;

pub struct SubscriberHandle(pub(crate) AbortHandle);

impl Drop for SubscriberHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}
