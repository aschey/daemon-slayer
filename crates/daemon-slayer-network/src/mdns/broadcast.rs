use std::collections::HashMap;
use std::fmt::Display;
use std::net::IpAddr;

use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::{BroadcastEventStore, EventStore};
use daemon_slayer_core::{BoxedError, FutureExt};
use futures::StreamExt;
use gethostname::gethostname;
use mdns_sd::{DaemonEvent, DaemonStatus, IfKind, ServiceDaemon, ServiceInfo, UnregisterStatus};
use recap::Recap;
use serde::{Deserialize, Serialize};
use tap::TapFallible;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::route_listener::RouteListenerService;
use crate::{get_default_interface, ServiceMetadata, ServiceProtocol};

#[derive(Deserialize, Serialize, Debug, Recap, Clone)]
#[recap(
    regex = r"^(?P<instance_name>[a-zA-Z0-9_-]+)\.((?P<subdomain>[a-zA-Z0-9_-]+)\._sub.)?_(?P<type_name>[a-zA-Z0-9_-]+)\._(?P<service_protocol>(?:tcp)|(?:udp))\.local\.$"
)]
pub struct MdnsBroadcastName {
    instance_name: String,
    subdomain: Option<String>,
    type_name: String,
    service_protocol: ServiceProtocol,
}

impl MdnsBroadcastName {
    pub fn new(
        instance_name: impl Into<String>,
        type_name: impl Into<String>,
        service_protocol: ServiceProtocol,
    ) -> Self {
        Self {
            subdomain: None,
            instance_name: instance_name.into(),
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

    pub fn subdomain(&self) -> Option<&str> {
        self.subdomain.as_deref()
    }

    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn service_protocol(&self) -> ServiceProtocol {
        self.service_protocol
    }

    pub fn service_type(&self) -> String {
        let service_type = format!("_{}._{}.local.", self.type_name, self.service_protocol);
        if let Some(subdomain) = &self.subdomain {
            format!("{subdomain}._sub.{service_type}")
        } else {
            service_type
        }
    }
}

impl Display for MdnsBroadcastName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}.{}", self.instance_name, self.service_type()))
    }
}

#[derive(Clone, Debug)]
pub enum MdnsBroadcastEvent {
    Announce {
        service_name: String,
        addresses: String,
    },
    IpAdd(IpAddr),
    IpDel(IpAddr),
    ParseIpAddrError(String),
    Error(String),
    Unregistered,
    RegistrationMissing,
    ShuttingDown,
    ShutDown,
}

pub struct MdnsBroadcastService {
    service_name: MdnsBroadcastName,
    event_tx: broadcast::Sender<MdnsBroadcastEvent>,
    broadcast_data: HashMap<String, String>,
    port: u16,
}

impl MdnsBroadcastService {
    pub fn new(
        service_name: MdnsBroadcastName,
        port: u16,
        broadcast_data: impl ServiceMetadata,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(32);
        Self {
            service_name,
            event_tx,
            broadcast_data: broadcast_data.metadata(),
            port,
        }
    }

    pub fn get_event_store(&self) -> BroadcastEventStore<MdnsBroadcastEvent> {
        BroadcastEventStore::new(self.event_tx.clone())
    }

    async fn register_service(&self, mdns: &ServiceDaemon) -> Result<String, BoxedError> {
        let (address, interface_name) = get_default_interface()
            .await?
            .map(|interface| (interface.ip().to_string(), interface.name))
            .unwrap_or_default();

        let hostname = format!("{}.local.", gethostname().to_string_lossy());

        let mut service_info = ServiceInfo::new(
            &self.service_name.service_type(),
            self.service_name.instance_name(),
            &hostname,
            &address,
            self.port,
            self.broadcast_data.clone(),
        )?;

        if address.is_empty() {
            service_info = service_info.enable_addr_auto();
        } else {
            info!("broadcasting on {address}");
            mdns.disable_interface(IfKind::All).unwrap();
            mdns.enable_interface(IfKind::Name(interface_name)).unwrap();
        }
        let service_fullname = service_info.get_fullname().to_owned();

        mdns.register(service_info)
            .expect("Failed to register mDNS service");

        Ok(service_fullname)
    }

    pub(crate) async fn get_monitor(&self) -> Result<(ServiceDaemon, String), BoxedError> {
        let mdns = ServiceDaemon::new()?;
        let service_fullname = self.register_service(&mdns).await?;
        Ok((mdns, service_fullname))
    }

    fn handle_mdns_event(&self, event: DaemonEvent) {
        info!("mdns register event: {event:?}");
        let res = match event {
            DaemonEvent::Announce(service_name, addresses) => {
                self.event_tx.send(MdnsBroadcastEvent::Announce {
                    service_name,
                    addresses,
                })
            }
            DaemonEvent::IpAdd(addr) => self.event_tx.send(MdnsBroadcastEvent::IpAdd(addr)),
            DaemonEvent::IpDel(addr) => self.event_tx.send(MdnsBroadcastEvent::IpDel(addr)),
            DaemonEvent::Error(mdns_sd::Error::ParseIpAddr(err)) => self
                .event_tx
                .send(MdnsBroadcastEvent::ParseIpAddrError(err)),
            DaemonEvent::Error(mdns_sd::Error::Msg(err)) => {
                self.event_tx.send(MdnsBroadcastEvent::Error(err))
            }
            _ => Ok(0),
        };
        res.tap_err(|e| warn!("error sending message: {e:?}")).ok();
    }

    pub(crate) async fn handle_route_change(
        &self,
        mdns: &ServiceDaemon,
        service_fullname: &str,
    ) -> Result<(), BoxedError> {
        info!("route change");
        let receiver = mdns.unregister(service_fullname).unwrap();
        while let Ok(event) = receiver.recv_async().await {
            info!("mdns unregister event: {event:?}");
            match event {
                UnregisterStatus::OK => {
                    self.event_tx.send(MdnsBroadcastEvent::Unregistered).ok();
                }
                UnregisterStatus::NotFound => {
                    self.event_tx
                        .send(MdnsBroadcastEvent::RegistrationMissing)
                        .ok();
                }
            }
        }

        self.register_service(mdns).await?;
        Ok(())
    }
}

impl BackgroundService for MdnsBroadcastService {
    fn name(&self) -> &str {
        "mdns_broadcast_service"
    }

    async fn run(self, mut context: ServiceContext) -> Result<(), BoxedError> {
        let (mdns, service_fullname) = self.get_monitor().await?;
        let monitor = mdns.monitor().unwrap();
        let route_service = RouteListenerService::new();
        let mut route_events = route_service.get_event_store().subscribe_events();
        context.add_service(route_service);

        let cancellation_token = context.cancellation_token();
        loop {
            tokio::select! {
                event = monitor.recv_async() => {
                    if let Ok(event) = event {
                        self.handle_mdns_event(event);
                    }
                }
                event = route_events.next() => {
                    if let Some(Ok(_)) = event {
                        self.handle_route_change(&mdns, &service_fullname).await?;
                    }
                }
                _ = cancellation_token.cancelled() => {
                    break;
                }
            }
        }

        while let Ok(Ok(event)) = monitor
            .recv_async()
            .cancel_on_shutdown(&context.cancellation_token())
            .await
        {
            self.handle_mdns_event(event)
        }

        let receiver = mdns.unregister(&service_fullname).unwrap();
        while let Ok(event) = receiver.recv_async().await {
            info!("mdns unregister event: {event:?}");
            match event {
                UnregisterStatus::OK => {
                    self.event_tx.send(MdnsBroadcastEvent::Unregistered).ok();
                }
                UnregisterStatus::NotFound => {
                    self.event_tx
                        .send(MdnsBroadcastEvent::RegistrationMissing)
                        .ok();
                }
            }
        }

        let shutdown_rx = mdns.shutdown().unwrap();
        while let Ok(event) = shutdown_rx.recv_async().await {
            match event {
                DaemonStatus::Running => {
                    self.event_tx.send(MdnsBroadcastEvent::ShuttingDown).ok();
                }
                DaemonStatus::Shutdown => {
                    self.event_tx.send(MdnsBroadcastEvent::ShutDown).ok();
                }
                _ => {}
            }
        }
        Ok(())
    }
}
