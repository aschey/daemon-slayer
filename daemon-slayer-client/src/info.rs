use crate::State;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Info {
    pub state: State,
    pub autostart: Option<bool>,
    pub pid: Option<u32>,
}
