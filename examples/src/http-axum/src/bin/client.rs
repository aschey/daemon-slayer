use std::error::Error;

use daemon_slayer::{
    cli::{clap, CliAsync, InputState},
    client::{HttpHealthCheckAsync, Manager, RequestType, ServiceManager},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let manager = ServiceManager::builder("daemon_slayer_axum")
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let command = clap::Command::default()
        .subcommand(clap::Command::new("hello").arg(clap::Arg::new("name")));

    let health_check = HttpHealthCheckAsync::new(RequestType::Get, "http://127.0.0.1:3000")?;
    let cli = CliAsync::client_builder(manager)
        .with_base_command(command)
        .with_health_check(Box::new(health_check))
        .build();

    if let InputState::Unhandled(matches) = cli.handle_input().await? {
        if let Some(("hello", args)) = matches.subcommand() {
            let unknown = "unknown".to_string();
            let name = args.get_one::<String>("name").unwrap_or(&unknown);
            let response = reqwest::get(format!("http://127.0.0.1:3000/hello/{name}")).await?;
            println!("{}", response.text().await?);
        }
    }

    Ok(())
}
