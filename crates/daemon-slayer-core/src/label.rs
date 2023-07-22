use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Eq, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Label {
    pub qualifier: String,
    pub organization: String,
    pub application: String,
}

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.qualified_name())
    }
}

impl Label {
    pub fn qualified_name(&self) -> String {
        format!(
            "{}.{}.{}",
            self.qualifier, self.organization, self.application
        )
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Identifier {0} was not in the correct format. Identifiers should be formatted as '{{qualifier}}.{{organization}}.{{application}}'.")]
    InvalidIdentifier(String),
}

impl FromStr for Label {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const IDENTIFIER_PARTS: usize = 3;
        let parts: Vec<_> = s.split('.').collect();
        if parts.len() != IDENTIFIER_PARTS {
            return Err(ParseError::InvalidIdentifier(s.to_owned()));
        }

        Ok(Label {
            qualifier: parts[0].to_owned(),
            organization: parts[1].to_owned(),
            application: parts[2].to_owned(),
        })
    }
}
