use clap::ArgMatches;

#[derive(Clone, PartialEq, Eq)]
pub enum InputState {
    Handled,
    Unhandled,
}
