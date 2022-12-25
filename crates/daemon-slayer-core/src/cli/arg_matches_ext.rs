use super::CommandType;
use clap::parser::ValueSource;

pub trait ArgMatchesExt
where
    Self: Sized,
{
    fn matches(&self, command_type: &CommandType) -> Option<Self>;
}

pub trait CommandExt {
    fn add_command_handler(self, command_type: &CommandType) -> Self;
}

fn contains_flags(matches: &clap::ArgMatches) -> bool {
    // Check if any flags were actually supplied, not just default values
    for arg_match in matches.ids() {
        if let Some(value_source) = matches.value_source(arg_match.as_str()) {
            if value_source != ValueSource::DefaultValue {
                return true;
            }
        }
    }
    false
}

impl ArgMatchesExt for clap::ArgMatches {
    fn matches(&self, command_type: &CommandType) -> Option<Self> {
        let contains_flags = contains_flags(self);
        match (command_type, self.subcommand()) {
            (CommandType::Arg { id, .. }, _)
                if self.value_source(id) == Some(ValueSource::CommandLine) =>
            {
                Some(self.clone())
            }
            (CommandType::Subcommand { name, .. }, Some((sub_name, sub))) if sub_name == name => {
                Some(sub.clone())
            }
            (CommandType::Default, None) if !contains_flags => Some(self.clone()),
            _ => None,
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
                let mut arg = clap::Arg::new(id)
                    .action(clap::ArgAction::SetTrue)
                    .hide(*hide);

                if let Some(short) = short {
                    arg = arg.short(*short);
                }
                if let Some(long) = long {
                    arg = arg.long(long);
                }
                if let Some(help_text) = help_text.as_ref() {
                    arg = arg.help(help_text);
                }
                self.arg(arg)
            }
            CommandType::Subcommand {
                name,
                help_text,
                hide,
                children,
            } => {
                let mut sub = clap::Command::new(name).about(help_text).hide(*hide);

                for child in children.iter() {
                    sub = sub.add_command_handler(child);
                }
                self.subcommand(sub)
            }
            CommandType::Default => self.arg_required_else_help(false),
        }
    }
}
