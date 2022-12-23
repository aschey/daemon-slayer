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

impl ArgMatchesExt for clap::ArgMatches {
    fn matches(&self, command_type: &CommandType) -> Option<Self> {
        match (command_type, self.subcommand()) {
            (CommandType::Arg { id, .. }, _)
                if self.value_source(id) == Some(ValueSource::CommandLine) =>
            {
                Some(self.clone())
            }
            (CommandType::Subcommand { name, .. }, Some((sub_name, sub))) if sub_name == name => {
                Some(sub.clone())
            }
            (CommandType::Default, None) if !self.args_present() => Some(self.clone()),
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
                children,
            } => {
                let mut sub = clap::Command::new(name).about(help_text).hide(*hide);

                if let Some(children) = children {
                    for child in children.iter() {
                        sub = sub.add_command_handler(child);
                    }
                }
                self.subcommand(sub)
            }
            CommandType::Default => self.arg_required_else_help(false),
        }
    }
}
