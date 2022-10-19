use clap::parser::ValueSource;

use super::CommandType;

pub trait ArgMatchesExt {
    fn matches(&self, command_type: &CommandType) -> bool;
}

pub trait CommandExt {
    fn add_command_handler(self, command_type: &CommandType) -> Self;
}

impl ArgMatchesExt for clap::ArgMatches {
    fn matches(&self, command_type: &CommandType) -> bool {
        match command_type {
            CommandType::Arg {
                id,
                short: _,
                long: _,
                help_text: _,
                hide: _,
            } => self.value_source(id) == Some(ValueSource::CommandLine),
            CommandType::Subcommand {
                name,
                help_text: _,
                hide: _,
            } => self.subcommand().map(|r| r.0) == Some(name),
            CommandType::Default => !self.args_present() && self.subcommand() == None,
        }
    }
}

impl CommandExt for clap::Command {
    fn add_command_handler(self, command_type: &CommandType) -> Self {
        match command_type {
            CommandType::Arg {
                id,
                short,
                long,
                help_text,
                hide,
            } => {
                let mut arg = clap::Arg::new(id);
                if let Some(short) = short {
                    arg = arg.short(*short);
                }
                if let Some(long) = long {
                    arg = arg.long(long);
                }

                self.arg(
                    arg.action(clap::ArgAction::SetTrue)
                        .help(help_text.as_ref().unwrap())
                        .hide(*hide),
                )
            }
            CommandType::Subcommand {
                name,
                help_text,
                hide,
            } => self.subcommand(clap::Command::new(name).about(help_text).hide(*hide)),
            CommandType::Default => self.arg_required_else_help(false),
        }
    }
}
