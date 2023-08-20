mod client;
mod handler;

pub use client::*;
pub use handler::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Signal {
    SIGTERM,
    SIGQUIT,
    SIGINT,
    SIGTSTP,
    SIGHUP,
    SIGCHLD,
    SIGCONT,
    Other(String),
}

impl From<&str> for Signal {
    fn from(source: &str) -> Self {
        match source.to_uppercase().as_ref() {
            "SIGTERM" => Signal::SIGTERM,
            "SIGQUIT" => Signal::SIGQUIT,
            "SIGINT" => Signal::SIGINT,
            "SIGTSTP" => Signal::SIGTSTP,
            "SIGHUP" => Signal::SIGHUP,
            "SIGCHLD" => Signal::SIGCHLD,
            "SIGCONT" => Signal::SIGCONT,
            _ => Signal::Other(source.to_string()),
        }
    }
}
