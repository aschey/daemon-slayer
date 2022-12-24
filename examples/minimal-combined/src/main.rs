use std::time::{Duration, Instant};

use clap::Parser;
use daemon_slayer::{
    client::{
        self,
        config::{
            windows::{ServiceAccess, Trustee, WindowsConfig},
            Level,
        },
    },
    core::{BoxedError, Label},
    server::{
        futures::StreamExt, BroadcastEventStore, EventStore, Handler, Service, ServiceContext,
        Signal, SignalHandler,
    },
    signals::SignalListener,
};

#[derive(clap::Parser, Debug)]
enum Arg {
    /// Run the service using the service manager
    Run,
    /// Install the service
    Install,
    /// Uninstall the service
    Uninstall,
    /// Retrieve information about the service status
    Info,
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
    let manager = client::builder(ServiceHandler::label())
        .with_description("test service")
        .with_args(["run"])
        .with_service_level(if cfg!(windows) {
            Level::System
        } else {
            Level::User
        })
        .with_windows_config(WindowsConfig::default().with_additional_access(
            Trustee::CurrentUser,
            ServiceAccess::Start | ServiceAccess::Stop | ServiceAccess::ChangeConfig,
        ))
        .build()?;

    match Cli::parse().arg {
        None => {
            ServiceHandler::run_directly(None).await?;
        }
        Some(Arg::Run) => {
            ServiceHandler::run_as_service(None).await?;
        }
        Some(Arg::Install) => {
            manager.install()?;
        }
        Some(Arg::Uninstall) => {
            manager.uninstall()?;
        }
        Some(Arg::Info) => {
            println!("{}", manager.info()?.pretty_print());
        }
        Some(Arg::Start) => {
            manager.start()?;
        }
        Some(Arg::Stop) => {
            manager.stop()?;
        }
        Some(Arg::Restart) => {
            manager.restart()?;
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
            .expect("Should parse the label")
    }

    async fn new(
        mut context: ServiceContext,
        _input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let signal_listener = SignalListener::all();
        let signal_store = signal_listener.get_event_store();
        context.add_service(signal_listener).await?;

        Ok(Self { signal_store })
    }

    async fn run_service<F: FnOnce() + Send>(mut self, on_started: F) -> Result<(), Self::Error> {
        println!("running service");
        let start_time = Instant::now();
        on_started();
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
