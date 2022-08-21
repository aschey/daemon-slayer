use crate::service_state::ServiceState;

pub trait ServiceManager {
    fn new<T: Into<String>>(service_name: T) -> Self;
    fn install<'a, T: Into<&'a str>>(&self, args: impl IntoIterator<Item = T>);
    fn uninstall(&self);
    fn start(&self);
    fn stop(&self);
    fn query_status(&self) -> ServiceState;
}
