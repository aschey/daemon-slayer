use std::{pin::Pin, time::Duration};

use daemon_slayer_core::BroadcastEventStore;
use futures::Future;
use tap::TapFallible;
use tracing::warn;

// use crate::Signal;

pub struct ServiceContext {
    //signal_tx: tokio::sync::broadcast::Sender<crate::Signal>,
    handles: Vec<Box<dyn FnOnce()>>,
}

impl ServiceContext {
    pub(crate) fn new() -> Self {
        Self {
            //   signal_tx,
            handles: vec![],
        }
    }
    pub fn add_event_service<S: daemon_slayer_core::blocking::EventService + 'static>(
        &mut self,
        builder: S::Builder,
    ) -> (S::Client, S::EventStoreImpl) {
        let mut service = S::run_service(builder);
        let client = service.get_client();
        let event_store = service.get_event_store();

        self.handles.push(Box::new(move || {
            service.stop();
        }));
        (client, event_store)
    }

    pub fn add_service<S: daemon_slayer_core::blocking::Service + 'static>(
        &mut self,
        builder: S::Builder,
    ) -> S::Client {
        let mut service = S::run_service(builder);
        let client = service.get_client();

        self.handles.push(Box::new(move || {
            service.stop();
        }));
        client
    }

    pub(crate) fn stop(self) {
        for handle in self.handles {
            handle();
        }
    }
}

pub struct ServiceContextSync {}
