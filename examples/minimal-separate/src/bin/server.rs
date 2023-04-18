use daemon_slayer::{
    core::{BoxedError, Label},
    server::{
        futures::StreamExt, BroadcastEventStore, EventStore, Handler, Service, ServiceContext,
        Signal, SignalHandler,
    },
    signals::SignalListener,
};
use std::time::{Duration, Instant};

#[tokio::main]
pub async fn main() -> Result<(), BoxedError> {
    let mut args = std::env::args();
    if let Some(arg) = args.next() {
        if arg == minimal_separate::run_argument().to_string() {
            ServiceHandler::run_as_service(None).await?;
            return Ok(());
        }
    }
    ServiceHandler::run_directly(None).await?;

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
        minimal_separate::label()
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

    async fn run_service<F: FnOnce() + Send>(mut self, notify_ready: F) -> Result<(), Self::Error> {
        println!("running service");
        notify_ready();

        let mut signal_rx = self.signal_store.subscribe_events();
        let start_time = Instant::now();
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
