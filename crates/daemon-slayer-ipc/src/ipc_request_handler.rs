use serde::{Deserialize, Serialize};

#[async_trait::async_trait]
pub trait IpcRequestHandler: Clone + Send {
    type Req: Serialize + for<'de> Deserialize<'de> + Unpin + Send;
    type Res: Serialize + for<'de> Deserialize<'de> + Unpin + Send;
    async fn handle_request(&mut self, request: Self::Req) -> Self::Res;
}
