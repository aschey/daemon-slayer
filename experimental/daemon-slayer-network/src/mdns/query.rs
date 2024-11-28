use std::fmt::Display;

use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::{BroadcastEventStore, EventStore};
use daemon_slayer_core::BoxedError;
use futures::StreamExt;
use mdns_sd::{DaemonStatus, IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use recap::Recap;
use serde::{Deserialize, Serialize};
use tap::TapFallible;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::route_listener::RouteListenerService;
use crate::{get_default_interface, ServiceProtocol};

#[derive(Deserialize, Serialize, Debug, Recap, Clone, PartialEq, Eq)]
#[recap(
    regex = r"^((?P<subdomain>[a-zA-Z0-9_-]+)\._sub.)?_(?P<type_name>[a-zA-Z0-9_-]+)\._(?P<service_protocol>(?:tcp)|(?:udp))\.local\.$"
)]
pub struct MdnsQueryName {
    subdomain: Option<String>,
    type_name: String,
    service_protocol: ServiceProtocol,
}

impl MdnsQueryName {
    pub fn new(type_name: impl Into<String>, service_protocol: ServiceProtocol) -> Self {
        Self {
            subdomain: None,
            type_name: type_name.into(),
            service_protocol,
        }
    }

    pub fn with_subdomain(self, subdomain: impl Into<String>) -> Self {
        Self {
            subdomain: Some(subdomain.into()),
            ..self
        }
    }
}

impl Display for MdnsQueryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = format!("_{}._{}.local.", self.type_name, self.service_protocol);
        if let Some(subdomain) = &self.subdomain {
            f.write_str(&format!("{subdomain}._sub.{name}"))
        } else {
            f.write_str(&name)
        }
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
    ShuttingDown,
    ShutDown,
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

    fn handle_service_event(&self, event: ServiceEvent) {
        info!("mdns query event: {event:?}");
        let res = match event {
            ServiceEvent::SearchStarted(service_type) => self
                .event_tx
                .send(MdnsReceiverEvent::SearchStarted(service_type)),
            ServiceEvent::ServiceFound(service_type, full_name) => {
                self.event_tx.send(MdnsReceiverEvent::ServiceFound {
                    service_type,
                    full_name,
                })
            }
            ServiceEvent::ServiceResolved(info) => {
                self.event_tx.send(MdnsReceiverEvent::ServiceResolved(info))
            }
            ServiceEvent::ServiceRemoved(service_type, full_name) => {
                self.event_tx.send(MdnsReceiverEvent::ServiceRemoved {
                    service_type,
                    full_name,
                })
            }
            ServiceEvent::SearchStopped(service_type) => self
                .event_tx
                .send(MdnsReceiverEvent::SearchStopped(service_type)),
        };
        res.tap_err(|e| warn!("failed to send message: {e:?}")).ok();
    }
}

impl BackgroundService for MdnsQueryService {
    fn name(&self) -> &str {
        "mdns_query_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let mdns = ServiceDaemon::new()?;

        if let Some(interface) = get_default_interface().await? {
            mdns.disable_interface(IfKind::All).unwrap();
            mdns.enable_interface(IfKind::Name(interface.name)).unwrap();
        }
        let receiver = mdns.browse(&self.service_name.to_string()).unwrap();

        let route_service = RouteListenerService::new();
        let mut route_events = route_service.get_event_store().subscribe_events();
        context.spawn(route_service);

        loop {
            tokio::select! {
                event = receiver.recv_async() => {
                    if let Ok(event) = event {
                        self.handle_service_event(event);
                    }
                }
                event = route_events.next() => {
                    if let Some(Ok(_)) = event {
                        info!("route change");
                        if let Some(interface) = get_default_interface().await? {
                            mdns.disable_interface(IfKind::All).unwrap();
                            mdns.enable_interface(IfKind::Name(interface.name)).unwrap();
                        } else {
                            mdns.enable_interface(IfKind::All).unwrap();
                        }
                    }
                }
                _ = context.cancelled() => {
                    break;
                }
            }
        }

        let shutdown_rx = mdns.shutdown().unwrap();
        while let Ok(event) = shutdown_rx.recv_async().await {
            match event {
                DaemonStatus::Running => {
                    self.event_tx.send(MdnsReceiverEvent::ShuttingDown).ok();
                }
                DaemonStatus::Shutdown => {
                    self.event_tx.send(MdnsReceiverEvent::ShutDown).ok();
                }
                _ => {}
            }
        }
        Ok(())
    }
}
