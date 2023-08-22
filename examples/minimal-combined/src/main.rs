use std::env::current_exe;
use std::time::{Duration, Instant};

use clap::Parser;
use daemon_slayer::client::config::windows::{ServiceAccess, Trustee, WindowsConfig};
use daemon_slayer::client::config::Level;
use daemon_slayer::client::{self};
use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::server::futures::StreamExt;
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, Service, ServiceContext, Signal, SignalHandler,
};
use daemon_slayer::signals::SignalListener;

#[derive(clap::Parser, Debug)]
enum Arg {
    /// Run the service using the service manager
    Run,
    /// Install the service
    Install,
    /// Uninstall the service
    Uninstall,
    /// Retrieve information about the service status
    Status,
    /// Start the service
    Start,
    /// Stop the service
    Stop,
    /// Restart the service
    Restart,
}

#[derive(clap::Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    arg: Option<Arg>,
}

#[tokio::main]
pub async fn main() -> Result<(), BoxedError> {
    let manager = client::builder(ServiceHandler::label(), current_exe()?.try_into()?)
        .with_description("test service")
        .with_arg(&"run".parse()?)
        .with_service_level(if cfg!(windows) {
            Level::System
        } else {
            Level::User
        })
        .with_windows_config(WindowsConfig::default().with_additional_access(
            Trustee::CurrentUser,
            ServiceAccess::Start | ServiceAccess::Stop | ServiceAccess::ChangeConfig,
        ))
        .build()
        .await?;

    match Cli::parse().arg {
        None => {
            ServiceHandler::run_directly(None).await?;
        }
        Some(Arg::Run) => {
            ServiceHandler::run_as_service(None).await?;
        }
        Some(Arg::Install) => {
            manager.install().await?;
        }
        Some(Arg::Uninstall) => {
            manager.uninstall().await?;
        }
        Some(Arg::Status) => {
            println!("{}", manager.status().await?.pretty_print());
        }
        Some(Arg::Start) => {
            manager.start().await?;
        }
        Some(Arg::Stop) => {
            manager.stop().await?;
        }
        Some(Arg::Restart) => {
            manager.restart().await?;
        }
    }

    Ok(())
}

#[derive(daemon_slayer::server::Service)]
pub struct ServiceHandler {
    signal_store: BroadcastEventStore<Signal>,
}

#[daemon_slayer::core::async_trait]
impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = ();

    fn label() -> Label {
        "com.example.daemon_slayer_minimal_combined"
            .parse()
            .expect("Failed to parse label")
    }

    async fn new(
        mut context: ServiceContext,
        _input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let signal_listener = SignalListener::termination();
        let signal_store = signal_listener.get_event_store();
        context.add_service(signal_listener);

        Ok(Self { signal_store })
    }

    async fn run_service<F: FnOnce() + Send>(mut self, notify_ready: F) -> Result<(), Self::Error> {
        println!("running service");
        let start_time = Instant::now();
        notify_ready();
        let mut signal_rx = self.signal_store.subscribe_events();
        loop {
            match tokio::time::timeout(Duration::from_secs(1), signal_rx.next()).await {
                Ok(_) => {
                    println!("stopping service");
                    return Ok(());
                }
                Err(_) => {
                    println!(
                        "Run time: {} seconds",
                        Instant::now().duration_since(start_time).as_secs()
                    )
                }
            }
        }
    }
}
