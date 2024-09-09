use std::time::{Duration, Instant};

use daemon_slayer::core::{BoxedError, Label};
use daemon_slayer::server::futures::StreamExt;
use daemon_slayer::server::{
    BroadcastEventStore, EventStore, Handler, Service, ServiceContext, Signal, SignalHandler,
};
use daemon_slayer::signals::SignalListener;

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

impl Handler for ServiceHandler {
    type Error = BoxedError;
    type InputData = ();

    fn label() -> Label {
        minimal_separate::label()
    }

    async fn new(
        context: ServiceContext,
        _input_data: Option<Self::InputData>,
    ) -> Result<Self, Self::Error> {
        let signal_listener = SignalListener::termination();
        let signal_store = signal_listener.get_event_store();
        context.spawn(signal_listener);

        Ok(Self { signal_store })
    }

    async fn run_service<F: FnOnce() + Send>(self, notify_ready: F) -> Result<(), Self::Error> {
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
