use std::error::Error;

use daemon_slayer::cli::Cli;
use daemon_slayer::client::{Manager, ServiceManager};
use daemon_slayer::error_handler::ErrorHandler;
use daemon_slayer::file_watcher::{FileWatcher, FileWatcherBuilder};
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{
    cli::ServerCliProvider, BroadcastEventStore, EventStore, Handler, Receiver, Service,
    ServiceContext,
};
use daemon_slayer::signals::{
    Signal, SignalHandler, SignalHandlerBuilder, SignalHandlerBuilderTrait,
};
use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use tracing::info;
use tracing::metadata::LevelFilter;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        info!("Got a request from {:?}", request.remote_addr());

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    daemon_slayer::logging::init_local_time();
    run_async()
}

#[tokio::main]
async fn run_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (logger, guard) = LoggerBuilder::for_server("daemon_slayer_tonic")
        .with_ipc_logger(true)
        .build()?;

    ErrorHandler::for_server().install()?;

    logger.init();
    let (mut cli, command) = Cli::builder()
        .with_provider(ServerCliProvider::<ServiceHandler>::default())
        .build();
    let matches = command.get_matches();
    cli.handle_input(&matches).await;
    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

#[tonic::async_trait]
impl Handler for ServiceHandler {
    async fn new(context: &mut ServiceContext) -> Self {
        let (_, signal_store) = context
            .add_event_service::<SignalHandler>(SignalHandlerBuilder::all())
            .await;

        Self { signal_store }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_tonic"
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");
        on_started();
        let addr = "[::1]:50052".parse().unwrap();
        let greeter = MyGreeter::default();

        println!("GreeterServer listening on {}", addr);

        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<GreeterServer<MyGreeter>>()
            .await;
        let mut shutdown_rx = self.signal_store.subscribe_events();
        Server::builder()
            .add_service(GreeterServer::new(greeter))
            .add_service(health_service)
            .serve_with_shutdown(addr, async {
                shutdown_rx.recv().await;
            })
            .await?;

        Ok(())
    }
}
