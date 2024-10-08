use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::EventStore;
use daemon_slayer_core::BoxedError;
use futures::StreamExt;
use tracing::info;

use super::DiscoveryProtocol;
use crate::mdns::{MdnsBroadcastName, MdnsBroadcastService};
use crate::route_listener::RouteListenerService;
use crate::udp::UdpBroadcastService;
use crate::{BroadcastServiceName, ServiceMetadata, ServiceProtocol};

enum DiscoveryImpl {
    Mdns(MdnsBroadcastService),
    Udp(UdpBroadcastService),
    Both(MdnsBroadcastService, UdpBroadcastService),
}

pub struct DiscoveryBroadcastService {
    discovery_impl: DiscoveryImpl,
}

impl DiscoveryBroadcastService {
    pub fn new(
        discovery_protocol: DiscoveryProtocol,
        service_name: BroadcastServiceName,
        service_protocol: ServiceProtocol,
        port: u16,
        broadcast_data: impl ServiceMetadata,
    ) -> Self {
        let mut mdns_name = MdnsBroadcastName::new(
            service_name.instance_name(),
            service_name.type_name(),
            service_protocol,
        );
        if let Some(subdomain) = service_name.subdomain() {
            mdns_name = mdns_name.with_subdomain(subdomain);
        }
        Self {
            discovery_impl: match discovery_protocol {
                DiscoveryProtocol::Mdns => {
                    DiscoveryImpl::Mdns(MdnsBroadcastService::new(mdns_name, port, broadcast_data))
                }
                DiscoveryProtocol::Udp { port } => DiscoveryImpl::Udp(
                    UdpBroadcastService::new(service_name, service_protocol, port, broadcast_data)
                        .with_broadcast_port(port),
                ),
                DiscoveryProtocol::Both { udp_port } => DiscoveryImpl::Both(
                    MdnsBroadcastService::new(mdns_name, port, broadcast_data.metadata()),
                    UdpBroadcastService::new(service_name, service_protocol, port, broadcast_data)
                        .with_broadcast_port(udp_port),
                ),
            },
        }
    }
}

async fn run_mdns(
    mdns_service: MdnsBroadcastService,
    context: ServiceContext,
) -> Result<(), BoxedError> {
    let route_service = RouteListenerService::new();
    let mut route_events = route_service.get_event_store().subscribe_events();
    context.spawn(route_service);

    let (mdns, service_fullname) = mdns_service.get_monitor().await?;
    let monitor = mdns.monitor().unwrap();

    loop {
        tokio::select! {
            event = monitor.recv_async() => {
                if let Ok(event) = event {
                    info!("mdns register event: {event:?}");
                }
            }
            event = route_events.next() => {
                if let Some(Ok(_)) = event {
                    mdns_service.handle_route_change(&mdns, &service_fullname).await?;
                }
            }
            _ = context.cancelled() => {
                break;
            }
        }
    }

    let receiver = mdns.unregister(&service_fullname).unwrap();
    while let Ok(event) = receiver.recv_async().await {
        info!("mdns unregister event: {event:?}");
    }

    let shutdown_rx = mdns.shutdown().unwrap();
    while let Ok(event) = shutdown_rx.recv_async().await {
        info!("mdns shutdown event: {event:?}");
    }

    Ok(())
}

impl BackgroundService for DiscoveryBroadcastService {
    fn name(&self) -> &str {
        "discovery_broadcast_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        match self.discovery_impl {
            DiscoveryImpl::Mdns(mdns_service) => {
                run_mdns(mdns_service, context).await?;
            }
            DiscoveryImpl::Udp(udp_service) => {
                udp_service.run(context.clone()).await?;
            }
            DiscoveryImpl::Both(mdns_service, udp_service) => {
                context.spawn((
                    "discovery_mdns_service",
                    move |context: ServiceContext| async move {
                        run_mdns(mdns_service, context).await
                    },
                ));
                context.spawn(udp_service);
            }
        }
        Ok(())
    }
}
