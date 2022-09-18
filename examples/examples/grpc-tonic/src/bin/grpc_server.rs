use std::error::Error;

use daemon_slayer::cli::CliAsync;
use daemon_slayer::logging::tracing_subscriber::util::SubscriberInitExt;
use daemon_slayer::logging::{LoggerBuilder, LoggerGuard};
use daemon_slayer::server::{EventHandlerAsync, HandlerAsync};
use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use tracing::info;

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
    let logger_builder = LoggerBuilder::new(ServiceHandler::get_service_name());
    run_async(logger_builder)
}

#[tokio::main]
async fn run_async(logger_builder: LoggerBuilder) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = CliAsync::for_server(
        ServiceHandler::new(),
        "daemon_slayer_tonic".to_owned(),
        "test_service".to_owned(),
    );

    let (logger, _guard) = logger_builder.with_ipc_logger(true).build().unwrap();

    logger.init();

    cli.handle_input().await?;
    Ok(())
}

#[derive(daemon_slayer::server::ServiceAsync)]
pub struct ServiceHandler {
    tx: tokio::sync::mpsc::Sender<()>,
    rx: tokio::sync::mpsc::Receiver<()>,
}

#[tonic::async_trait]
impl HandlerAsync for ServiceHandler {
    fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        Self { tx, rx }
    }

    fn get_service_name<'a>() -> &'a str {
        "daemon_slayer_async_server"
    }

    fn get_event_handler(&mut self) -> EventHandlerAsync {
        let tx = self.tx.clone();
        Box::new(move |event| {
            let tx = tx.clone();
            Box::pin(async move {
                info!("stopping");
                let _ = tx.send(()).await;
                Ok(())
            })
        })
    }

    async fn run_service<F: FnOnce() + Send>(
        mut self,
        on_started: F,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("running service");
        on_started();
        let addr = "[::1]:50051".parse().unwrap();
        let greeter = MyGreeter::default();

        println!("GreeterServer listening on {}", addr);

        let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
        health_reporter
            .set_serving::<GreeterServer<MyGreeter>>()
            .await;

        Server::builder()
            .add_service(GreeterServer::new(greeter))
            .add_service(health_service)
            .serve_with_shutdown(addr, async { self.rx.recv().await.unwrap_or_default() })
            .await?;

        Ok(())
    }
}
