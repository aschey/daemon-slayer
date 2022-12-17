use serde::{Deserialize, Serialize};
use tarpc::{server::Serve, transport::channel::UnboundedChannel, ClientMessage, Response};

pub trait ServiceProvider: Clone + Send + Sync + 'static {
    type Resp: Serialize + for<'de> Deserialize<'de> + Send + Unpin + 'static;
    type Req: Serialize + for<'de> Deserialize<'de> + Send + Unpin + 'static;
    type Client: Clone + Send + 'static;

    type Service: Serve<Self::Req, Resp = Self::Resp> + Send + Clone + 'static;

    fn get_service(&self, client: Self::Client) -> Self::Service;
    fn get_client(
        &self,
        chan: UnboundedChannel<Response<Self::Resp>, ClientMessage<Self::Req>>,
    ) -> Self::Client;
}
