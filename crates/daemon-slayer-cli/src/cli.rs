use crate::Builder;
use daemon_slayer_core::{
    cli::{ActionType, ArgMatchesExt, CommandMatch, CommandProvider, InputState},
    BoxedError,
};

pub struct Cli {
    providers: Vec<Box<dyn CommandProvider>>,
    matches: clap::ArgMatches,
    matched_command: Option<CommandMatch>,
}

impl Cli {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub(crate) fn new(
        mut providers: Vec<Box<dyn CommandProvider>>,
        matches: clap::ArgMatches,
    ) -> Result<Self, BoxedError> {
        let mut matched_command: Option<CommandMatch> = None;
        for provider in &providers {
            for cmd in provider.get_commands() {
                if let Some(matches) = matches.matches(&cmd.command_type) {
                    matched_command = Some(CommandMatch {
                        matched_command: cmd.to_owned(),
                        matches,
                    });
                }
            }
        }
        for provider in &mut providers {
            provider.initialize(&matches, &matched_command)?;
        }
        Ok(Self {
            providers,
            matches,
            matched_command,
        })
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

    pub fn get_provider<T: CommandProvider>(&mut self) -> Option<&mut T> {
        self.providers
            .iter_mut()
            .find_map(|p| p.as_any_mut().downcast_mut::<T>())
    }

    pub async fn handle_input(self) -> Result<(InputState, clap::ArgMatches), BoxedError> {
        for provider in self.providers {
            if provider
                .handle_input(&self.matches, &self.matched_command)
                .await?
                == InputState::Handled
            {
                return Ok((InputState::Handled, self.matches));
            }
        }
        Ok((InputState::Unhandled, self.matches))
    }
}

#[cfg(test)]
#[path = "./cli_test.rs"]
mod cli_test;
