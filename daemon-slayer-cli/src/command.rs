pub enum Command {
    Subcommand {
        name: String,
        help_text: String,
    },
    Arg {
        short: Option<char>,
        long: Option<String>,
        help_text: Option<String>,
    },
    Default,
}
