use clap::{Arg, ArgAction};

use crate::{command::Command, commands::Commands, service_commands::ServiceCommands};

pub(crate) fn build_cmd<'a>(
    display_name: &'a str,
    description: &'a str,
    commands: impl Iterator<Item = (&'a &'a str, &'a Command)>,
) -> clap::Command<'a> {
    let mut cmd = clap::Command::new(display_name).about(description);
    for (name, command) in commands {
        let mut hide = false;
        #[cfg(feature = "server")]
        {
            hide = (*name) == ServiceCommands::RUN;
        }

        match command {
            Command::Arg {
                short,
                long,
                help_text,
            } => {
                let mut arg = Arg::new(*name);
                if let Some(short) = short {
                    arg = arg.short(*short);
                }
                if let Some(long) = long {
                    arg = arg.long(long);
                }

                cmd = cmd.arg(
                    arg.action(ArgAction::SetTrue)
                        .help(help_text.as_ref().map(&String::as_ref))
                        .hide(hide),
                )
            }
            Command::Subcommand { name, help_text } => {
                cmd = cmd.subcommand(clap::command!(name).about(&**help_text).hide(hide))
            }
            Command::Default => {}
        }
    }
    cmd
}
