use std::collections::HashMap;

use daemon_slayer_core::cli::{Action, ActionType, CommandProvider, CommandType, InputState};

use crate::Builder;

pub struct Cli {
    providers: Vec<Box<dyn CommandProvider>>,
    matches: clap::ArgMatches,
}

impl Cli {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub(crate) fn new(
        mut providers: Vec<Box<dyn CommandProvider>>,
        matches: clap::ArgMatches,
    ) -> Self {
        for provider in &mut providers {
            provider.initialize(&matches);
        }
        Self { providers, matches }
    }

    pub fn action_type(&self, matches: &clap::ArgMatches) -> ActionType {
        for provider in &self.providers {
            let action_type = provider.action_type(matches);
            if action_type != ActionType::Unknown {
                return action_type;
            }
        }
        ActionType::Unknown
    }

    pub fn get_matches(&self) -> &clap::ArgMatches {
        &self.matches
    }

    pub async fn handle_input(self) -> (InputState, clap::ArgMatches) {
        for provider in self.providers {
            if provider.handle_input(&self.matches).await == InputState::Handled {
                return (InputState::Handled, self.matches);
            }
        }
        (InputState::Unhandled, self.matches)
    }
}
