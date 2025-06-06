use std::any::TypeId;

use clap::builder::StyledStr;
use daemon_slayer_core::BoxedError;
use daemon_slayer_core::cli::{ActionType, CommandMatch, CommandProvider, InputState};

use crate::Builder;

pub struct Cli {
    providers: Vec<Box<dyn CommandProvider>>,
    matches: clap::ArgMatches,
    help: StyledStr,
    matched_command: Option<(CommandMatch, TypeId)>,
}

impl Cli {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub(crate) fn new(
        mut providers: Vec<Box<dyn CommandProvider>>,
        help: StyledStr,
        matches: clap::ArgMatches,
    ) -> Result<Self, BoxedError> {
        let mut matched_command: Option<(CommandMatch, TypeId)> = None;

        for provider in &mut providers {
            if let Some(command_match) = provider.matches(&matches) {
                matched_command = Some((command_match, provider.type_id()));

                break;
            }
        }

        for provider in &mut providers {
            provider.initialize(&matches, matched_command.as_ref().map(|(cmd, _)| cmd))?;
        }
        Ok(Self {
            providers,
            matches,
            help,
            matched_command,
        })
    }

    pub fn action_type(&self) -> ActionType {
        if let Some((cmd, _)) = &self.matched_command {
            cmd.action_type.clone()
        } else {
            ActionType::Unknown
        }
    }

    pub fn get_matches(&self) -> &clap::ArgMatches {
        &self.matches
    }

    pub fn try_get_provider<T: CommandProvider>(&mut self) -> Option<&mut T> {
        self.providers
            .iter_mut()
            .find_map(|p| p.as_any_mut().downcast_mut::<T>())
    }

    pub fn get_provider<T: CommandProvider>(&mut self) -> &mut T {
        self.try_get_provider().expect("Provider not found")
    }

    pub fn try_take_provider<T: CommandProvider>(&mut self) -> Option<T> {
        let provider_index = self
            .providers
            .iter()
            .position(|p| p.as_any().downcast_ref::<T>().is_some());
        if let Some(i) = provider_index {
            match self.providers.remove(i).downcast() {
                Ok(provider) => Some(*provider),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    pub fn take_provider<T: CommandProvider>(&mut self) -> T {
        self.try_take_provider().expect("Provider not found")
    }

    pub async fn handle_input(self) -> Result<(InputState, clap::ArgMatches), BoxedError> {
        self.handle_input_with_writer(std::io::stdout()).await
    }

    pub async fn handle_input_with_writer(
        mut self,
        mut writer: impl std::io::Write + Send + Sync,
    ) -> Result<(InputState, clap::ArgMatches), BoxedError> {
        if let Some((_, provider_type)) = self.matched_command {
            let provider_index = self
                .providers
                .iter()
                .position(|p| p.type_id() == provider_type)
                .unwrap();
            let provider = self.providers.remove(provider_index);
            let handler_result = provider.handle_input().await?;

            if let Some(output) = handler_result.output {
                writeln!(writer, "{output}")?;
            }

            match handler_result.input_state {
                InputState::Handled => return Ok((InputState::Handled, self.matches)),
                InputState::UsageError(message) => {
                    writeln!(writer, "{message}")?;
                    writeln!(writer, "{}", self.help)?;
                    return Ok((InputState::UsageError(message), self.matches));
                }
                _ => {}
            }
        }
        Ok((InputState::Unhandled, self.matches))
    }
}

#[cfg(test)]
#[path = "./cli_test.rs"]
mod cli_test;
