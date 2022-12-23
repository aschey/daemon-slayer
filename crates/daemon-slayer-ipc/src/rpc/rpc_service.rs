use daemon_slayer_core::{
    server::{BackgroundService, ServiceContext},
    BoxedError, FutureExt,
};
use futures::{future, StreamExt};
use parity_tokio_ipc::{Endpoint, SecurityAttributes};
use tarpc::server::{BaseChannel, Channel, Serve};

use crate::{
    build_transport, get_socket_address, ipc_client_stream::IpcClientStream, two_way::spawn_twoway,
    Codec, CodecWrapper,
};

use super::ServiceProvider;

pub struct RpcService<P: ServiceProvider>
where
    <<P as ServiceProvider>::Service as Serve<<P as ServiceProvider>::Req>>::Fut: Send,
{
    bind_addr: String,
    service_provider: P,
    codec: Codec,
}

impl<P: ServiceProvider> RpcService<P>
where
    <<P as ServiceProvider>::Service as Serve<<P as ServiceProvider>::Req>>::Fut: Send,
{
    pub fn new(id: &str, service_provider: P, codec: Codec) -> Self {
        Self {
            bind_addr: get_socket_address(id, "rpc"),
            service_provider,
            codec,
        }
    }

    pub fn get_client(&self) -> P::Client {
        let conn = IpcClientStream::new(self.bind_addr.clone());

        let (server_chan, client_chan) =
            spawn_twoway(build_transport(conn, CodecWrapper::new(self.codec.clone())));

        let peer = self.service_provider.get_client(client_chan);
        let peer_ = peer.clone();
        let service_factory = self.service_provider.clone();
        tokio::spawn(async move {
            let service = service_factory.get_service(peer_);
            BaseChannel::with_defaults(server_chan)
                .execute(service)
                .await;
        });
        peer
    }
}

#[async_trait::async_trait]
impl<P> BackgroundService for RpcService<P>
where
    P: ServiceProvider + Send,
    <<P as ServiceProvider>::Service as Serve<<P as ServiceProvider>::Req>>::Fut: Send,
{
    fn name<'a>() -> &'a str {
        "rpc_service"
    }

    async fn run(mut self, context: ServiceContext) -> Result<(), BoxedError> {
        let mut endpoint = Endpoint::new(self.bind_addr.clone());
        endpoint.set_security_attributes(SecurityAttributes::allow_everyone_create().unwrap());

        let incoming = endpoint.incoming().expect("failed to open new socket");
        let service_provider = self.service_provider.clone();
        let codec = self.codec.clone();
        incoming
            .filter_map(|r| future::ready(r.ok()))
            .map(|stream| {
                let (server_chan, client_chan) =
                    spawn_twoway(build_transport(stream, CodecWrapper::new(codec.clone())));

                let peer = service_provider.get_client(client_chan);
                (BaseChannel::with_defaults(server_chan), peer)
            })
            .map(|(base_chan, peer)| base_chan.execute(service_provider.get_service(peer)))
            .buffer_unordered(10)
            .for_each(|_| async {})
            .cancel_on_shutdown(&context.cancellation_token())
            .await
            .ok();
        Ok(())
    }
}
