use daemon_slayer_core::server::background_service::{BackgroundService, ServiceContext};
use daemon_slayer_core::{async_trait, BoxedError, CancellationToken, FutureExt};
use tracing::info;

use super::DiscoveryProtocol;
use crate::mdns::{MdnsBroadcastName, MdnsBroadcastService};
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
                DiscoveryProtocol::Udp => DiscoveryImpl::Udp(UdpBroadcastService::new(
                    service_name,
                    service_protocol,
                    port,
                    broadcast_data,
                )),
                DiscoveryProtocol::Both => DiscoveryImpl::Both(
                    MdnsBroadcastService::new(mdns_name, port, broadcast_data.metadata()),
                    UdpBroadcastService::new(service_name, service_protocol, port, broadcast_data),
                ),
            },
        }
    }
}

async fn run_mdns(
    mdns_service: MdnsBroadcastService,
    cancellation_token: CancellationToken,
) -> Result<(), BoxedError> {
    let (mdns, service_fullname) = mdns_service.get_monitor().await?;
    let monitor = mdns.monitor().unwrap();

    while let Ok(Ok(event)) = monitor
        .recv_async()
        .cancel_on_shutdown(&cancellation_token)
        .await
    {
        info!("mdns register event: {event:?}");
    }

    let receiver = mdns.unregister(&service_fullname).unwrap();
    while let Ok(event) = receiver.recv_async().await {
        info!("mdns unregister event: {event:?}");
    }

    Ok(())
}

#[async_trait]
impl BackgroundService for DiscoveryBroadcastService {
    fn name(&self) -> &str {
        "discovery_broadcast_service"
    }

    async fn run(mut self, mut context: ServiceContext) -> Result<(), BoxedError> {
        match self.discovery_impl {
            DiscoveryImpl::Mdns(mdns_service) => {
                run_mdns(mdns_service, context.cancellation_token()).await?;
            }
            DiscoveryImpl::Udp(udp_service) => {
                udp_service.run(context.clone()).await?;
            }
            DiscoveryImpl::Both(mdns_service, udp_service) => {
                context.add_service((
                    "discovery_mdns_service",
                    move |context: ServiceContext| async move {
                        run_mdns(mdns_service, context.cancellation_token()).await
                    },
                ));
                context.add_service(udp_service);
            }
        }
        Ok(())
    }
}
