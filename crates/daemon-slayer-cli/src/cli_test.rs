use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use clap::{Args, FromArgMatches, Subcommand};
use daemon_slayer_core::{
    async_trait,
    cli::{ActionType, CommandMatch, CommandOutput, CommandProvider, InputState},
    BoxedError,
};

use crate::Cli;

#[test]
fn test_initialize() {
    let mut cli = Cli::builder()
        .with_base_command(clap::Command::new("cli_test"))
        .with_provider(TestProvider::new(
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ))
        .initialize()
        .unwrap();
    let provider = cli.get_provider::<TestProvider>().unwrap();
    assert!(provider.initialized);
}

#[tokio::test]
async fn test_input_handled_default() {
    let default_bool = Arc::new(AtomicBool::new(false));
    let cli = Cli::builder()
        .with_base_command(clap::Command::new("cli_test"))
        .with_provider(TestProvider::new(
            default_bool.clone(),
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ))
        .initialize()
        .unwrap();
    let (input_state, _) = cli.handle_input().await.unwrap();
    assert_eq!(InputState::Handled, input_state);
    assert!(default_bool.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_input_handled_subcommand() {
    let subcommand_bool = Arc::new(AtomicBool::new(false));
    let cli = Cli::builder()
        .with_base_command(clap::Command::new("cli_test"))
        .with_provider(TestProvider::new(
            Arc::new(AtomicBool::new(false)),
            subcommand_bool.clone(),
            Arc::new(AtomicBool::new(false)),
        ))
        .initialize_from(["cli_test", "test"])
        .unwrap();
    let (input_state, _) = cli.handle_input().await.unwrap();
    assert_eq!(InputState::Handled, input_state);
    assert!(subcommand_bool.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_output_subcommand() {
    let cli = Cli::builder()
        .with_base_command(clap::Command::new("cli_test"))
        .with_provider(TestProvider::new(
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ))
        .initialize_from(["cli_test", "test"])
        .unwrap();
    let mut buf = Vec::new();
    cli.handle_input_with_writer(&mut buf).await.unwrap();
    assert_eq!("subcommand\n", String::from_utf8(buf).unwrap());
}

#[tokio::test]
async fn test_input_handled_arg() {
    let arg_bool = Arc::new(AtomicBool::new(false));
    let cli = Cli::builder()
        .with_base_command(clap::Command::new("cli_test"))
        .with_provider(TestProvider::new(
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            arg_bool.clone(),
        ))
        .initialize_from(["cli_test", "-t", "true"])
        .unwrap();
    let (input_state, _) = cli.handle_input().await.unwrap();
    assert_eq!(InputState::Handled, input_state);
    assert!(arg_bool.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_base_command() {
    let subcommand_bool = Arc::new(AtomicBool::new(false));
    let cli = Cli::builder()
        .with_base_command(clap::Command::new("cli_test").subcommand(clap::Command::new("test2")))
        .with_provider(TestProvider::new(
            Arc::new(AtomicBool::new(false)),
            subcommand_bool.clone(),
            Arc::new(AtomicBool::new(false)),
        ))
        .initialize_from(["cli_test", "test2"])
        .unwrap();
    let (input_state, matches) = cli.handle_input().await.unwrap();
    assert_eq!(InputState::Unhandled, input_state);
    assert!(!subcommand_bool.load(Ordering::Relaxed));
    assert_eq!("test2", matches.subcommand().unwrap().0);
}

#[tokio::test]
async fn test_action_type() {
    let cli = Cli::builder()
        .with_base_command(clap::Command::new("cli_test"))
        .with_provider(TestProvider::new(
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ))
        .initialize_from(["cli_test", "test"])
        .unwrap();

    assert_eq!(ActionType::Client, cli.action_type());
}

#[tokio::test]
async fn test_action_type_unhandled() {
    let cli = Cli::builder()
        .with_base_command(clap::Command::new("cli_test").subcommand(clap::Command::new("test2")))
        .with_provider(TestProvider::new(
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ))
        .initialize_from(["cli_test", "test2"])
        .unwrap();

    assert_eq!(ActionType::Unknown, cli.action_type());
}

#[derive(Subcommand)]
enum TestCommands {
    Test,
}

#[derive(Args)]
struct TestArgs {
    #[arg(short, long)]
    test_arg: Option<bool>,
}

struct TestProvider {
    initialized: bool,
    default_matched: Arc<AtomicBool>,
    subcommand_matched: Arc<AtomicBool>,
    arg_matched: Arc<AtomicBool>,
}

impl TestProvider {
    fn new(
        default_matched: Arc<AtomicBool>,
        subcommand_matched: Arc<AtomicBool>,
        arg_matched: Arc<AtomicBool>,
    ) -> Self {
        Self {
            initialized: false,
            default_matched,
            subcommand_matched,
            arg_matched,
        }
    }
}

#[async_trait]
impl CommandProvider for TestProvider {
    fn get_commands(&self, command: clap::Command) -> clap::Command {
        let command = TestCommands::augment_subcommands(command);
        TestArgs::augment_args(command)
    }

    async fn handle_input(
        self: Box<Self>,
        matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<CommandOutput, BoxedError> {
        if matches.subcommand().is_none() && !matches.args_present() {
            self.default_matched.store(true, Ordering::Relaxed);
            return Ok(CommandOutput::handled(None));
        }
        if TestCommands::from_arg_matches(matches).is_ok() {
            self.subcommand_matched.store(true, Ordering::Relaxed);
            return Ok(CommandOutput::handled("subcommand".to_owned()));
        }

        if matches!(
            TestArgs::from_arg_matches(matches),
            Ok(TestArgs { test_arg: Some(_) })
        ) {
            self.arg_matched.store(true, Ordering::Relaxed);
            return Ok(CommandOutput::handled(None));
        }

        Ok(CommandOutput::unhandled())
    }

    fn matches(&self, matches: &clap::ArgMatches) -> Option<CommandMatch> {
        let arg_match = matches!(
            TestArgs::from_arg_matches(matches),
            Ok(TestArgs { test_arg: Some(_) })
        );
        let cmd_match = TestCommands::from_arg_matches(matches).is_ok();
        if arg_match || cmd_match {
            Some(CommandMatch {
                action_type: ActionType::Client,
                action: None,
            })
        } else {
            None
        }
    }

    fn initialize(
        &mut self,
        _matches: &clap::ArgMatches,
        _matched_command: &Option<CommandMatch>,
    ) -> Result<(), BoxedError> {
        self.initialized = true;
        Ok(())
    }
}
