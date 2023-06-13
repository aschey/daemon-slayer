use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, Args, FromArgMatches, Subcommand},
        ActionType, CommandMatch, CommandOutput, CommandProvider,
    },
    notify::BlockingNotification,
    BoxedError, Label,
};
use native_dialog::MessageType;
use tap::TapFallible;
use tracing::error;

use super::{Alert, Confirm, MessageDialog};

#[derive(Subcommand)]
enum DialogCommand {
    Alert(DialogArgs),
    Confirm(DialogArgs),
}

#[derive(Args, Clone)]
struct DialogArgs {
    #[arg(short, long)]
    title: Option<String>,
    text: String,
    #[arg(short, long, value_parser = message_type_parser)]
    message_type: Option<MessageType>,
}

fn message_type_parser(val: &str) -> Result<MessageType, String> {
    match val.to_lowercase().as_str() {
        "info" => Ok(MessageType::Info),
        "warn" | "warning" => Ok(MessageType::Warning),
        "error" => Ok(MessageType::Error),
        other => Err(format!("Invalid message type {other}")),
    }
}

pub struct DialogCliProvider {
    label: Label,
}

impl DialogCliProvider {
    pub fn new(label: Label) -> Self {
        Self { label }
    }
}

#[async_trait]
impl CommandProvider for DialogCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        DialogCommand::augment_subcommands(command)
    }

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        DialogCommand::from_arg_matches(matches).ok()?;
        Some(CommandMatch {
            action_type: ActionType::Server,
            action: None,
        })
    }

    async fn handle_input(
        mut self: Box<Self>,
        matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        let Ok(command) = DialogCommand::from_arg_matches(matches) else {
            return Ok(CommandOutput::unhandled());
        };

        match command {
            DialogCommand::Alert(args) => {
                let mut dialog = MessageDialog::<Alert>::new(self.label);
                if let Some(title) = args.title {
                    dialog = dialog.with_title(title);
                }
                dialog = dialog.with_text(args.text);

                dialog
                    .show_blocking()
                    .tap_err(|e| error!("Error showing dialog: {e}"))
                    .ok();
                Ok(CommandOutput::handled(None))
            }
            DialogCommand::Confirm(args) => {
                let mut dialog = MessageDialog::<Confirm>::new(self.label);
                if let Some(title) = args.title {
                    dialog = dialog.with_title(title);
                }
                dialog = dialog.with_text(args.text);

                let result = dialog.show_blocking();
                Ok(CommandOutput::handled(
                    result
                        .map(|r| r.to_string())
                        .unwrap_or_else(|e| e.to_string()),
                ))
            }
        }
    }
}
