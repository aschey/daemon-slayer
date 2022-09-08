use clap::{Arg, ArgAction, ArgMatches};

use crate::{command::Command, commands::Commands, service_commands::ServiceCommands};

pub(crate) fn matches(m: &ArgMatches, cmd: &Command, cmd_name: &'static str) -> bool {
    match cmd {
        Command::Arg {
            short: _,
            long: _,
            help_text: _,
        } => m.get_one::<bool>(cmd_name) == Some(&true),
        Command::Subcommand {
            name: _,
            help_text: _,
        } => m.subcommand().map(|r| r.0) == Some(cmd_name),
        Command::Default => !m.args_present() && m.subcommand() == None,
    }
}
