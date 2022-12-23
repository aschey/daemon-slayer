#[derive(Clone)]
pub enum CommandType {
    Subcommand {
        name: String,
        help_text: String,
        hide: bool,
        children: Option<Vec<CommandType>>,
    },
    Arg {
        id: String,
        short: Option<char>,
        long: Option<String>,
        help_text: Option<String>,
        hide: bool,
    },
    Default,
}