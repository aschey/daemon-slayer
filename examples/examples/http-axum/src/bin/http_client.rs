use std::{env::current_exe, error::Error, path::PathBuf};

use daemon_slayer::{
    cli::{
        clap::{Arg, Command},
        Cli, InputState,
    },
    client::{
        cli::ClientCliProvider,
        config::{ServiceAccess, Trustee, WindowsConfig},
        Level, Manager, ServiceManager,
    },
    console::{cli::ConsoleCliProvider, Console},
    error_handler::ErrorHandler,
    health_check::{cli::HealthCheckCliProvider, HttpHealthCheck, HttpRequestType},
    logging::tracing_subscriber::util::SubscriberInitExt,
};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (logger, _guard) =
        daemon_slayer::logging::LoggerBuilder::for_client("daemon_slayer_axum").build()?;
    logger.init();
    ErrorHandler::for_client().install()?;

    let manager = ServiceManager::builder("daemon_slayer_axum")
        .with_description("test service")
        .with_program(current_exe().unwrap().parent().unwrap().join("http_server"))
        .with_service_level(if cfg!(windows) {
            Level::System
        } else {
            Level::User
        })
        .with_windows_config(WindowsConfig::default().with_additional_access(
            Trustee::CurrentUser,
            ServiceAccess::Start | ServiceAccess::Stop | ServiceAccess::ChangeConfig,
        ))
        .with_args(["run"])
        .build()?;

    let command = Command::default()
        .subcommand(Command::new("hello").arg(Arg::new("name")))
        .arg_required_else_help(true)
        .about("Send a request to the server");
    let health_check = HttpHealthCheck::new(HttpRequestType::Get, "http://127.0.0.1:3000/health")?;

    let mut console = Console::new(manager.clone());
    console.add_health_check(Box::new(health_check.clone()));
    let (cli, command) = Cli::builder()
        .with_base_command(command)
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(HealthCheckCliProvider::new(health_check))
        .build();

    let matches = command.get_matches();

    if let InputState::Unhandled = cli.handle_input(&matches).await {
        if let Some(("hello", args)) = matches.subcommand() {
            let unknown = "unknown".to_string();
            let name = args.get_one::<String>("name").unwrap_or(&unknown);
            let response = reqwest::get(format!("http://127.0.0.1:3000/hello/{name}")).await?;
            println!("{}", response.text().await?);
        }
    }

    Ok(())
}
