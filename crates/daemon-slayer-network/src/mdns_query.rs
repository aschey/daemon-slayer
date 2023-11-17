use std::fmt::Display;
use std::time::Duration;

use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::BroadcastEventStore;
use daemon_slayer_core::{async_trait, BoxedError, FutureExt};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use recap::Recap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::ServiceProtocol;

#[derive(Deserialize, Serialize, Debug, Recap)]
#[recap(
    regex = r"^_(?P<type_name>[a-zA-Z0-9_-]+)\._(?P<service_protocol>(?:tcp)|(?:udp))\.local\.$"
)]
pub struct MdnsQueryName {
    type_name: String,
    service_protocol: ServiceProtocol,
}

impl MdnsQueryName {
    pub fn new(type_name: String, service_protocol: ServiceProtocol) -> Self {
        Self {
            type_name,
            service_protocol,
        }
    }
}

impl Display for MdnsQueryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "_{}._{}.local.",
            self.type_name, self.service_protocol
        ))
    }
}

#[derive(Clone, Debug)]
pub enum MdnsReceiverEvent {
    SearchStarted(String),
    ServiceFound {
        service_type: String,
        full_name: String,
    },
    ServiceResolved(ServiceInfo),
    ServiceRemoved {
        service_type: String,
        full_name: String,
    },
    SearchStopped(String),
}

pub struct MdnsQueryService {
    service_name: MdnsQueryName,
    event_tx: broadcast::Sender<MdnsReceiverEvent>,
}

impl MdnsQueryService {
    pub fn new(service_name: MdnsQueryName) -> Self {
        let (event_tx, _) = broadcast::channel(32);
        Self {
            service_name,
            event_tx,
        }
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<MdnsReceiverEvent> {
        BroadcastEventStore::new(self.event_tx.clone())
    }
}

#[async_trait]
impl BackgroundService for MdnsQueryService {
    fn shutdown_timeout() -> Duration {
        Duration::from_secs(1)
    }

    fn name(&self) -> &str {
        "mdns_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let mdns = ServiceDaemon::new().unwrap();
        let receiver = mdns.browse(&self.service_name.to_string()).unwrap();

        while let Ok(Ok(event)) = receiver
            .recv_async()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            match event {
                ServiceEvent::SearchStarted(service_type) => {
                    self.event_tx
                        .send(MdnsReceiverEvent::SearchStarted(service_type))
                        .ok();
                }
                ServiceEvent::ServiceFound(service_type, full_name) => {
                    self.event_tx
                        .send(MdnsReceiverEvent::ServiceFound {
                            service_type,
                            full_name,
                        })
                        .ok();
                }
                ServiceEvent::ServiceResolved(info) => {
                    self.event_tx
                        .send(MdnsReceiverEvent::ServiceResolved(info))
                        .ok();
                }
                ServiceEvent::ServiceRemoved(service_type, full_name) => {
                    self.event_tx
                        .send(MdnsReceiverEvent::ServiceRemoved {
                            service_type,
                            full_name,
                        })
                        .ok();
                }
                ServiceEvent::SearchStopped(service_type) => {
                    self.event_tx
                        .send(MdnsReceiverEvent::SearchStopped(service_type))
                        .ok();
                }
            }
        }
        Ok(())
    }
}
