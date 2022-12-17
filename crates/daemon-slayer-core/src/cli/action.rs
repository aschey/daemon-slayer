use strum_macros::{Display, EnumProperty, EnumString};

use super::ActionType;

#[derive(Display, Clone, PartialEq, Eq, Hash, Debug, EnumString, EnumProperty)]
#[strum(serialize_all = "kebab-case")]
pub enum Action {
    #[strum(props(Type = "client"))]
    Install,
    #[strum(props(Type = "client"))]
    Uninstall,
    #[strum(props(Type = "client"))]
    Info,
    #[strum(props(Type = "client"))]
    Start,
    #[strum(props(Type = "client"))]
    Stop,
    #[strum(props(Type = "client"))]
    Restart,
    #[strum(props(Type = "client"))]
    Reload,
    #[strum(props(Type = "client"))]
    Enable,
    #[strum(props(Type = "client"))]
    Disable,
    #[strum(props(Type = "client"))]
    Pid,
    #[strum(props(Type = "server"))]
    Run,
    #[strum(props(Type = "server"))]
    Direct,
}

impl Action {
    pub fn action_type(&self) -> ActionType {
        use strum::EnumProperty;
        match self.get_str("Type").unwrap() {
            "client" => ActionType::Client,
            "server" => ActionType::Server,
            _ => unreachable!("invalid action type"),
        }
    }
}
