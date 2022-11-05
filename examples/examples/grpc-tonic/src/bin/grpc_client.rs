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
    error_handler::{cli::ErrorHandlerCliProvider, ErrorHandler},
    health_check::{cli::HealthCheckCliProvider, GrpcHealthCheck},
    logging::{cli::LoggingCliProvider, tracing_subscriber::util::SubscriberInitExt},
};
use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
async fn run_async() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let logger_builder = daemon_slayer::logging::LoggerBuilder::new("daemon_slayer_tonic");
    let logging_provider = LoggingCliProvider::new(logger_builder);

    let manager = ServiceManager::builder("daemon_slayer_tonic")
        .with_description("test service")
        .with_program(current_exe().unwrap().parent().unwrap().join("grpc_server"))
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
    let health_check = GrpcHealthCheck::new("http://[::1]:50052")?;

    let mut console = Console::new(manager.clone());
    console.add_health_check(Box::new(health_check.clone()));
    let cli = Cli::builder()
        .with_base_command(command)
        .with_provider(ClientCliProvider::new(manager.clone()))
        .with_provider(ConsoleCliProvider::new(console))
        .with_provider(HealthCheckCliProvider::new(health_check))
        .with_provider(logging_provider.clone())
        .with_provider(ErrorHandlerCliProvider::default())
        .initialize();

    let (logger, _guard) = logging_provider.get_logger();
    logger.init();

    if let (InputState::Unhandled, matches) = cli.handle_input().await {
        if let Some(("hello", args)) = matches.subcommand() {
            let mut client = GreeterClient::connect("http://[::1]:50052").await?;

            let request = tonic::Request::new(HelloRequest {
                name: args
                    .get_one::<String>("name")
                    .unwrap_or(&"unknown".to_owned())
                    .into(),
            });

            let response = client.say_hello(request).await?;

            println!("{}", response.into_inner().message);
        }
    }

    Ok(())
}
