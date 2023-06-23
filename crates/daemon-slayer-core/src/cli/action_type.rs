#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ActionType {
    Server,
    Client,
    Other,
    Unknown,
}
