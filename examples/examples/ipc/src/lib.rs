use daemon_slayer::ipc::rpc::{RpcService, ServiceProvider};
use daemon_slayer::ipc::Codec;
use daemon_slayer::ipc::IpcRequestHandler;
use std::sync::Arc;
use tarpc::context::{self, Context};
use tarpc::{client, transport::channel::UnboundedChannel, ClientMessage, Response};
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IpcRequest {
    pub name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IpcResponse {
    pub message: String,
}

#[derive(Clone)]
pub struct RequestHandler {}

#[async_trait::async_trait]
impl IpcRequestHandler for RequestHandler {
    type Req = IpcRequest;
    type Res = IpcResponse;

    async fn handle_request(&mut self, request: Self::Req) -> Self::Res {
        info!("Got request: {request:?}");
        IpcResponse {
            message: format!("hello {}", request.name),
        }
    }
}
