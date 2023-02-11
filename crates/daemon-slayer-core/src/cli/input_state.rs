#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputState {
    Handled,
    Unhandled,
    UsageError(String),
}

pub struct CommandOutput {
    pub input_state: InputState,
    pub output: Option<String>,
}

impl CommandOutput {
    pub fn unhandled() -> Self {
        Self {
            input_state: InputState::Unhandled,
            output: None,
        }
    }

    pub fn usage_error(message: impl Into<String>) -> Self {
        Self {
            input_state: InputState::UsageError(message.into()),
            output: None,
        }
    }

    pub fn handled(output: impl Into<Option<String>>) -> Self {
        Self {
            input_state: InputState::Handled,
            output: output.into(),
        }
    }
}
