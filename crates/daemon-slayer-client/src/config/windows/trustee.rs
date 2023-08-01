use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum Trustee {
    CurrentUser,
    Name(String),
}

impl Display for Trustee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrentUser => f.write_str("Current User"),
            Self::Name(name) => f.write_str(name),
        }
    }
}
