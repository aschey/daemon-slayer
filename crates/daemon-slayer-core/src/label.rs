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
    #[error("Identifier {0} was not in the correct format. Identifiers should be formatted as '{{qualifier}}.{{organization}}.{{application}}'. The first two identifiers are optional.")]
    InvalidIdentifier(String),
}

impl FromStr for Label {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let max_identifier_parts = 3;
        let parts: Vec<_> = s.split('.').collect();
        if parts.len() > max_identifier_parts {
            return Err(ParseError::InvalidIdentifier(s.to_owned()));
        }

        let (qualifier, organization, application) = match parts.len() {
            0 => ("", "", ""),
            1 => ("", "", parts[0]),
            2 => ("", parts[0], parts[1]),
            3 => (parts[0], parts[1], parts[2]),
            _ => unreachable!("Should've verified max length"),
        };

        Ok(Label {
            qualifier: qualifier.to_owned(),
            organization: organization.to_owned(),
            application: application.to_owned(),
        })
    }
}
