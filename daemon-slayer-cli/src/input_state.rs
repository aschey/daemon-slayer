use clap::ArgMatches;

pub enum InputState {
    Handled,
    Unhandled(ArgMatches),
}
