use crate::service_command::ServiceCommand;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ActionType {
    Server,
    Client,
    Unknown,
}

// pub struct Action {
//     pub action_type: ActionType,
//     pub command: Option<String>,
// }
