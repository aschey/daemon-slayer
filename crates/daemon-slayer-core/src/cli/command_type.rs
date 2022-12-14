#[derive(Clone, Debug)]
pub enum CommandType {
    Subcommand {
        name: String,
        help_text: String,
        hide: bool,
        children: Vec<CommandType>,
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
