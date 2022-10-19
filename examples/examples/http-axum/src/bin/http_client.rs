use std::error::Error;

use daemon_slayer::{
    cli::clap,
    client::{
        health_check::{HttpHealthCheckAsync, HttpRequestType},
        Manager, ServiceManager,
    },
    logging::tracing_subscriber::util::SubscriberInitExt,
};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ServiceManager::builder("daemon_slayer_axum")
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let command = clap::Command::default()
        .subcommand(clap::Command::new("hello").arg(clap::Arg::new("name")))
        .arg_required_else_help(true)
        .about("Send a request to the server");

    let (mut cli, command) = daemon_slayer::cli::Cli::builder()
        .with_base_command(command)
        .with_provider(daemon_slayer::client::cli::ClientCliProvider::new(
            manager.clone(),
        ))
        .with_provider(daemon_slayer::console::cli::ConsoleCliProvider::new(
            manager,
        ))
        .build();

    let matches = command.get_matches();

    //    let health_check = HttpHealthCheckAsync::new(HttpRequestType::Get, "http://127.0.0.1:3000")?;
    // let cli = CliAsync::builder_for_client(manager)
    //     .with_base_command(command)
    //     .with_health_check(Box::new(health_check))
    //     .build();
    // let (logger, _guard) = cli.configure_logger().build()?;
    // logger.init();
    // cli.configure_error_handler().install()?;

    if let daemon_slayer::cli::InputState::Unhandled = cli.handle_input(&matches).await {
        if let Some(("hello", args)) = matches.subcommand() {
            let unknown = "unknown".to_string();
            let name = args.get_one::<String>("name").unwrap_or(&unknown);
            let response = reqwest::get(format!("http://127.0.0.1:3000/hello/{name}")).await?;
            println!("{}", response.text().await?);
        }
    }

    Ok(())
}
