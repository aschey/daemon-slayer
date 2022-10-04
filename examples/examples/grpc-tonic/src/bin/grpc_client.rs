use daemon_slayer::cli::{clap, CliAsync, InputState};
use daemon_slayer::client::{health_check::GrpcHealthCheckAsync, Manager, ServiceManager};
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
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
    let manager = ServiceManager::builder("daemon_slayer_async_server")
        .with_description("test service")
        .with_args(["run"])
        .build()
        .unwrap();

    let command = clap::Command::default().subcommand(
        clap::Command::new("hello")
            .arg(clap::Arg::new("name"))
            .about("send a request to the server"),
    );

    let health_check = GrpcHealthCheckAsync::new("http://[::1]:50051")?;
    let cli = CliAsync::builder_for_client(manager)
        .with_base_command(command)
        .with_health_check(Box::new(health_check))
        .build();

    let (logger, _guard) = cli.configure_logger().build()?;
    logger.init();

    cli.configure_error_handler().install()?;

    if let InputState::Unhandled(matches) = cli.handle_input().await? {
        if let Some(("hello", args)) = matches.subcommand() {
            let mut client = GreeterClient::connect("http://[::1]:50051").await?;

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
