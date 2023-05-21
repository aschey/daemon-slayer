use daemon_slayer_core::{
    async_trait,
    cli::{
        clap::{self, Args, FromArgMatches, Subcommand},
        ActionType, CommandMatch, CommandOutput, CommandProvider,
    },
    notify::Notification,
    BoxedError, Label,
};

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
}

impl NotifyCliProvider {
    pub fn new(label: Label) -> Self {
        Self { label }
    }
}

#[async_trait]
impl CommandProvider for NotifyCliProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        NotifyCommand::augment_subcommands(command)
    }

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        NotifyCommand::from_arg_matches(matches).ok()?;
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
        let Ok(NotifyCommand::Notify(args)) = NotifyCommand::from_arg_matches(matches) else {
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
        notification.show().await?;

        Ok(CommandOutput::handled("Notification sent".to_owned()))
    }
}
