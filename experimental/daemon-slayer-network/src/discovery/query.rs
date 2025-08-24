use std::collections::HashMap;

use daemon_slayer_core::BoxedError;
use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::server::tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use daemon_slayer_core::server::{BroadcastEventStore, DedupeEventStore, EventStore};
use futures::StreamExt;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use tokio::sync::broadcast;
use tokio_util::future::FutureExt;
use tracing::info;

use super::DiscoveryProtocol;
use crate::mdns::{MdnsBroadcastName, MdnsQueryName};
use crate::udp::UdpQueryService;
use crate::{BroadcastServiceName, QueryServiceName, ServiceInfo, ServiceProtocol};

enum DiscoveryImpl {
    Mdns(MdnsQueryName),
    Udp(UdpQueryService),
    Both(MdnsQueryName, UdpQueryService),
}

pub struct DiscoveryQueryService {
    event_tx: broadcast::Sender<ServiceInfo>,
    service_name: QueryServiceName,
    discovery_impl: DiscoveryImpl,
    service_protocol: ServiceProtocol,
}

impl DiscoveryQueryService {
    pub fn new(
        discovery_protocol: DiscoveryProtocol,
        service_name: QueryServiceName,
        service_protocol: ServiceProtocol,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(32);
        let mut mdns_name = MdnsQueryName::new(service_name.type_name(), service_protocol);
        if let Some(subdomain) = service_name.subdomain() {
            mdns_name = mdns_name.with_subdomain(subdomain);
        }
        Self {
            service_name,
            event_tx,
            service_protocol,
            discovery_impl: match discovery_protocol {
                DiscoveryProtocol::Mdns => DiscoveryImpl::Mdns(mdns_name),
                DiscoveryProtocol::Udp { port } => {
                    DiscoveryImpl::Udp(UdpQueryService::new().with_broadcast_port(port))
                }
                DiscoveryProtocol::Both { udp_port } => DiscoveryImpl::Both(
                    mdns_name,
                    UdpQueryService::new().with_broadcast_port(udp_port),
                ),
            },
        }
    }

    pub fn get_event_store(
        &self,
    ) -> impl EventStore<Item = Result<ServiceInfo, BroadcastStreamRecvError>> + use<> {
        DedupeEventStore::new(BroadcastEventStore::new(self.event_tx.clone()))
    }
}

async fn run_mdns(
    sender: broadcast::Sender<ServiceInfo>,
    mdns_name: MdnsQueryName,
    context: ServiceContext,
) -> Result<(), BoxedError> {
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.browse(&mdns_name.to_string()).unwrap();

    while let Some(Ok(event)) = receiver
        .recv_async()
        .with_cancellation_token(context.cancellation_token())
        .await
    {
        info!("mdns receiver event: {event:?}");
        if let ServiceEvent::ServiceResolved(info) = event {
            let mdns_broadcast_name: MdnsBroadcastName = info.get_fullname().parse().unwrap();
            let mut broadcast_service_name = BroadcastServiceName::new(
                mdns_broadcast_name.instance_name(),
                mdns_broadcast_name.type_name(),
            );
            if let Some(subdomain) = mdns_broadcast_name.subdomain() {
                broadcast_service_name = broadcast_service_name.with_subdomain(subdomain);
            }
            sender
                .send(ServiceInfo {
                    host_name: info.get_hostname().trim_end_matches('.').to_string(),
                    service_name: broadcast_service_name,
                    service_protocol: mdns_broadcast_name.service_protocol(),
                    port: info.get_port(),
                    ip_addresses: info.get_addresses().to_owned(),
                    broadcast_data: HashMap::from_iter(
                        info.get_properties()
                            .iter()
                            .map(|p| (p.key().to_owned(), p.val_str().to_owned())),
                    ),
                })
                .unwrap();
        }
    }

    let shutdown_rx = mdns.shutdown().unwrap();
    while let Ok(event) = shutdown_rx.recv_async().await {
        info!("mdns shutdown event: {event:?}");
    }

    Ok(())
}

async fn run_udp(
    sender: broadcast::Sender<ServiceInfo>,
    udp_query_service: UdpQueryService,
    search_service_name: QueryServiceName,
    service_protocol: ServiceProtocol,
    context: ServiceContext,
) -> Result<(), BoxedError> {
    let mut framed = udp_query_service.get_framed().await;

    let mut last_result = ServiceInfo::default();
    while let Some(Ok(service_info)) = framed
        .next()
        .with_cancellation_token(context.cancellation_token())
        .await
        .flatten()
    {
        let subdomain_matches = match (
            service_info.service_name.subdomain(),
            search_service_name.subdomain(),
        ) {
            (Some(current_sub), Some(search_sub)) => current_sub == search_sub,
            (None, Some(_)) => false,
            _ => true,
        };
        if service_info != last_result
            && service_info.service_name.type_name == search_service_name.type_name()
            && service_info.service_protocol == service_protocol
            && subdomain_matches
        {
            sender.send(service_info.clone()).unwrap();
            last_result = service_info;
        }
    }

    Ok(())
}

impl BackgroundService for DiscoveryQueryService {
    fn name(&self) -> &str {
        "discovery_query_service"
    }

    async fn run(self, context: ServiceContext) -> Result<(), BoxedError> {
        let sender = self.event_tx.clone();
        match self.discovery_impl {
            DiscoveryImpl::Mdns(mdns_name) => {
                run_mdns(sender, mdns_name, context.clone()).await?;
            }
            DiscoveryImpl::Udp(udp_service) => {
                run_udp(
                    sender,
                    udp_service,
                    self.service_name,
                    self.service_protocol,
                    context.clone(),
                )
                .await?
            }
            DiscoveryImpl::Both(mdns_name, udp_service) => {
                let sender_ = sender.clone();

                context.spawn((
                    "discovery_mdns_service",
                    move |context: ServiceContext| async move {
                        run_mdns(sender_, mdns_name, context.clone()).await?;
                        Ok(())
                    },
                ));
                let service_name = self.service_name;
                let service_protocol = self.service_protocol;
                context.spawn((
                    "discovery_udp_service",
                    move |context: ServiceContext| async move {
                        run_udp(
                            sender,
                            udp_service,
                            service_name,
                            service_protocol,
                            context.clone(),
                        )
                        .await?;
                        Ok(())
                    },
                ));
            }
        }
        Ok(())
    }
}
