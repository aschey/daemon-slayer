#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Started,
    Stopped,
    NotInstalled,
}
