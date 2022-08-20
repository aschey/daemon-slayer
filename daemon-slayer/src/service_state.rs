#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    Started,
    Stopped,
    NotInstalled,
}
