use crate::State;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Info {
    pub state: State,
    pub autostart: Option<bool>,
    pub pid: Option<u32>,
    pub last_exit_code: Option<i32>,
}
