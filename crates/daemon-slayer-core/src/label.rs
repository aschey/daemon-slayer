use std::str::FromStr;

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct Label {
    pub qualifier: String,
    pub organization: String,
    pub application: String,
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
    #[error("Identifier {0} was not in the correct format. Identifiers should be formatted as '{{qualifier}}.{{organization}}.{{application}}'")]
    InvalidIdentifier(String),
}

impl FromStr for Label {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let identifier_parts = 3;
        let parts: Vec<_> = s.split('.').collect();
        if parts.len() != identifier_parts {
            return Err(ParseError::InvalidIdentifier(s.to_owned()));
        }

        Ok(Label {
            qualifier: parts[0].to_owned(),
            organization: parts[1].to_owned(),
            application: parts[2].to_owned(),
        })
    }
}
