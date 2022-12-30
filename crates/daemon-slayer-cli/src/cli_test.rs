use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use daemon_slayer_core::{
    async_trait,
    cli::{ActionType, CommandConfig, CommandMatch, CommandProvider, CommandType, InputState},
    BoxedError,
};

use crate::Cli;

#[test]
fn test_initialize() {
    let mut cli = Cli::builder()
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
async fn test_input_handled_arg() {
    let arg_bool = Arc::new(AtomicBool::new(false));
    let cli = Cli::builder()
        .with_provider(TestProvider::new(
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            arg_bool.clone(),
        ))
        .initialize_from(["cli_test", "-t"])
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

struct TestProvider {
    initialized: bool,
    default_matched: Arc<AtomicBool>,
    subcommand_matched: Arc<AtomicBool>,
    arg_matched: Arc<AtomicBool>,
    commands: Vec<CommandConfig>,
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
            commands: vec![
                CommandConfig {
                    action_type: ActionType::Client,
                    command_type: CommandType::Default,
                    action: None,
                },
                CommandConfig {
                    action_type: ActionType::Client,
                    command_type: CommandType::Subcommand {
                        name: "test".to_owned(),
                        help_text: "help".to_owned(),
                        hide: false,
                        children: vec![],
                    },
                    action: None,
                },
                CommandConfig {
                    action_type: ActionType::Client,
                    command_type: CommandType::Arg {
                        id: "test_arg".to_owned(),
                        short: Some('t'),
                        long: Some("test".to_owned()),
                        help_text: None,
                        hide: false,
                    },
                    action: None,
                },
            ],
        }
    }
}

#[async_trait]
impl CommandProvider for TestProvider {
    fn get_commands(&self) -> Vec<&CommandConfig> {
        self.commands.iter().collect()
    }

    async fn handle_input(
        self: Box<Self>,
        _matches: &clap::ArgMatches,
        matched_command: &Option<CommandMatch>,
    ) -> Result<InputState, BoxedError> {
        if let Some(matched) = matched_command {
            return match &matched.matched_command.command_type {
                CommandType::Default => {
                    self.default_matched.store(true, Ordering::Relaxed);
                    Ok(InputState::Handled)
                }
                CommandType::Subcommand { name, .. } if name.as_str() == "test" => {
                    self.subcommand_matched.store(true, Ordering::Relaxed);
                    Ok(InputState::Handled)
                }
                CommandType::Arg { id, .. } if id.as_str() == "test_arg" => {
                    self.arg_matched.store(true, Ordering::Relaxed);
                    Ok(InputState::Handled)
                }
                _ => Ok(InputState::Unhandled),
            };
        }
        Ok(InputState::Unhandled)
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
