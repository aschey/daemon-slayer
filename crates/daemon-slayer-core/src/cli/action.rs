use strum_macros::{Display, EnumString};

#[derive(Display, Clone, PartialEq, Eq, Hash, Debug)]
#[strum(serialize_all = "kebab-case")]
pub enum Action {
    Server(ServerAction),
    Client(ClientAction),
}

#[derive(Display, Clone, PartialEq, Eq, Hash, Debug, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum ServerAction {
    Run,
    Direct,
}

#[derive(Display, Clone, PartialEq, Eq, Hash, Debug, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum ClientAction {
    Install,
    Uninstall,
    Info,
    Start,
    Stop,
    Restart,
    Reload,
    Enable,
    Disable,
    Pid,
    Status,
}
