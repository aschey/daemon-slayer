use daemon_slayer_core::cli::clap::{self, Args, FromArgMatches, Subcommand};
use daemon_slayer_core::cli::{ActionType, CommandMatch, CommandOutput, CommandProvider};
use daemon_slayer_core::notify::AsyncNotification;
use daemon_slayer_core::{async_trait, BoxedError, Label};

use crate::notification::Notification;

#[derive(Subcommand)]
enum NotifyCommand {
    Notify(NotifyArgs),
}

#[derive(Args, Clone)]
struct NotifyArgs {
    summary: String,
    #[arg(short, long)]
    subtitle: Option<String>,
    #[arg(short, long)]
    body: Option<String>,
    #[arg(short, long)]
    image_path: Option<String>,
    #[arg(long)]
    sound_name: Option<String>,
    #[arg(long)]
    icon: Option<String>,
}

pub struct NotifyCliProvider {
    label: Label,
    args: Option<NotifyArgs>,
}

impl NotifyCliProvider {
    pub fn new(label: Label) -> Self {
        Self { label, args: None }
    }
}

#[async_trait]
impl CommandProvider for NotifyCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        NotifyCommand::augment_subcommands(command)
    }

    fn matches(&mut self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let command = NotifyCommand::from_arg_matches(matches).ok()?;
        let NotifyCommand::Notify(args) = command;
        self.args = Some(args);
        Some(CommandMatch {
            action_type: ActionType::Other,
            action: None,
        })
    }

    async fn handle_input(mut self: Box<Self>) -> Result<CommandOutput, BoxedError> {
        let Some(args) = self.args else {
            return Ok(CommandOutput::unhandled());
        };

        let mut notification = Notification::new(self.label).summary(args.summary);
        if let Some(subtitle) = args.subtitle {
            notification = notification.subtitle(subtitle);
        }
        if let Some(body) = args.body {
            notification = notification.body(body);
        }
        if let Some(image_path) = args.image_path {
            notification = notification.image_path(image_path);
        }
        if let Some(sound_name) = args.sound_name {
            notification = notification.sound_name(sound_name);
        }
        if let Some(icon) = args.icon {
            notification = notification.icon(icon);
        }
        let output = notification
            .show()
            .await
            .map(|_| "Notification sent".to_owned())
            .unwrap_or_else(|e| e.to_string());

        Ok(CommandOutput::handled(output))
    }
}
